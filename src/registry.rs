//! Ontology registry with TTL-based eviction and on-demand reload.
//!
//! The registry tracks the *currently loaded* ontology (a single active slot
//! at a time) along with the metadata needed to reload it cheaply from the
//! N-Triples compile cache.
//!
//! Lifecycle:
//!   1. `load_file(path, name, opts)` — parses (or reads cache), populates
//!      `Arc<GraphStore>`, records active entry.
//!   2. `touch()` — called by every read tool (`onto_query`, `onto_stats`, ...)
//!      to keep the active entry alive.
//!   3. Background `evictor_tick()` clears the in-memory store if the active
//!      entry has been idle for longer than `idle_ttl_secs`.
//!   4. `ensure_loaded()` — called by every read tool *before* using the graph;
//!      if the store was evicted, it reloads from the cache (or from source if
//!      `auto_refresh` detected a change).
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::cache::{CacheManager, SourceFingerprint};
use crate::config::CacheConfig;
use crate::graph::GraphStore;
use crate::ontology::TRIPLE_COUNT_KEY;
use crate::state::StateDb;

/// RDF serialization format used for the compile cache (N-Triples).
/// Passed to `GraphStore::serialize` and friends wherever the cache is written.
///
/// # Examples
///
/// ```
/// use open_ontologies::registry::NTRIPLES_FORMAT;
/// assert_eq!(NTRIPLES_FORMAT, "ntriples");
/// ```
pub const NTRIPLES_FORMAT: &str = "ntriples";

/// RDF serialization format string for Turtle (`.ttl`).
/// Passed to `GraphStore::serialize` and `GraphStore::save_file` at 3 sites in
/// server.rs; a typo here produces an unserializable artifact silently.
///
/// # Examples
///
/// ```
/// use open_ontologies::registry::TURTLE_FORMAT;
/// assert_eq!(TURTLE_FORMAT, "turtle");
/// ```
pub const TURTLE_FORMAT: &str = "turtle";

/// Options accepted by `OntologyRegistry::load_file`.
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    pub name: Option<String>,
    /// Re-parse the source on every `ensure_loaded` if its mtime/sha changed.
    pub auto_refresh: bool,
    /// Bypass the cache and re-parse from source unconditionally.
    pub force_recompile: bool,
}

/// Outcome of a load operation, returned to callers (and surfaced in tool JSON).
#[derive(Debug, Clone)]
pub struct LoadResult {
    pub name: String,
    pub source_path: String,
    pub triple_count: usize,
    /// "cache" if loaded from N-Triples cache, "source" if re-parsed.
    pub origin: &'static str,
    pub cache_path: String,
}

#[derive(Debug)]
struct ActiveEntry {
    name: String,
    source_path: String,
    fingerprint: SourceFingerprint,
    cache_path: PathBuf,
    auto_refresh: bool,
    /// `Mutex<Instant>` to allow `&self`-touch from many threads.
    last_access: Mutex<Instant>,
    /// True after the in-memory store has been cleared by the evictor.
    /// Read tools must reload before using the graph.
    evicted: Mutex<bool>,
}

use std::marker::PhantomData;

pub struct Unbound;
pub struct Bound;

/// Registry of executable boundaries for the AutoReceipt pipeline.
pub struct ExecutionRegistry<State = Unbound> {
    _state: PhantomData<State>,
    pub bindings: std::collections::HashMap<String, String>,
}

impl ExecutionRegistry<Unbound> {
    pub fn new() -> Self {
        Self {
            _state: PhantomData,
            bindings: std::collections::HashMap::new(),
        }
    }

    pub fn bind(mut self, component: &str, target: &str) -> Self {
        self.bindings.insert(component.to_string(), target.to_string());
        self
    }

    pub fn transition(self) -> ExecutionRegistry<Bound> {
        ExecutionRegistry {
            _state: PhantomData,
            bindings: self.bindings,
        }
    }
}

impl ExecutionRegistry<Bound> {
    pub fn resolve(&self, component: &str) -> Option<&String> {
        self.bindings.get(component)
    }
}

/// Registry of the active ontology. Wrapped in `Arc` and shared with the server.
pub struct OntologyRegistry {
    graph: Arc<GraphStore>,
    cache: CacheManager,
    config: CacheConfig,
    /// `Mutex<Option<ActiveEntry>>` — at most one entry is "active" at a time
    /// in this minimal model. Multi-slot extension is future work.
    active: Mutex<Option<ActiveEntry>>,
    /// Per-name reload mutex to prevent thundering-herd reloads.
    reload_lock: Mutex<()>,
}

impl OntologyRegistry {
    pub fn new(graph: Arc<GraphStore>, db: StateDb, config: CacheConfig) -> Result<Self> {
        let cache_dir = PathBuf::from(crate::config::expand_tilde(&config.dir));
        let cache = CacheManager::new(cache_dir, db)?;
        Ok(Self {
            graph,
            cache,
            config,
            active: Mutex::new(None),
            reload_lock: Mutex::new(()),
        })
    }

    /// Return the effective cache configuration for this registry.
    ///
    /// # Examples
    ///
    /// A freshly constructed registry with a temporary cache directory exposes
    /// the configuration that was passed to [`OntologyRegistry::new`]:
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use open_ontologies::graph::GraphStore;
    /// # use open_ontologies::state::StateDb;
    /// # use open_ontologies::registry::OntologyRegistry;
    /// # use open_ontologies::config::CacheConfig;
    /// # let tmp = tempfile::tempdir().unwrap();
    /// # let store = Arc::new(GraphStore::new());
    /// # let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// # let config = CacheConfig {
    /// #     dir: tmp.path().to_string_lossy().into_owned(),
    /// #     ..CacheConfig::default()
    /// # };
    /// # let reg = OntologyRegistry::new(store, db, config).unwrap();
    /// let cfg = reg.config();
    /// assert!(cfg.evictor_interval_secs >= 1);
    /// ```
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    pub fn cache(&self) -> &CacheManager {
        &self.cache
    }

    /// Load an RDF file into the graph, using the compile cache when valid.
    pub fn load_file(&self, path: &str, opts: LoadOptions) -> Result<LoadResult> {
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(anyhow!("file not found: {}", path));
        }
        let name = opts.name.clone().unwrap_or_else(|| crate::cache::derive_name(path));
        let fp = SourceFingerprint::from_path(path_obj)?;

        // Decide load strategy: cache vs source.
        let existing = self.cache.get(&name)?;
        let cache_is_fresh = !opts.force_recompile
            && self.config.enabled
            && existing
                .as_ref()
                .map(|e| {
                    e.source_path == path
                        && e.source_mtime == fp.mtime_secs
                        && e.source_size == fp.size
                        && e.source_sha == fp.sha_prefix
                        && Path::new(&e.cache_path).exists()
                })
                .unwrap_or(false);

        // Always start from a clean store for a fresh load.
        self.graph.clear()?;

        let (triple_count, origin, cache_path) = if cache_is_fresh {
            let entry = existing.unwrap();
            let nt = std::fs::read_to_string(&entry.cache_path)
                .with_context(|| format!("read cache {}", entry.cache_path))?;
            let count = self.graph.load_ntriples(&nt)?;
            self.cache.touch(&name)?;
            (count, "cache", PathBuf::from(entry.cache_path))
        } else {
            let count = self
                .graph
                .load_file(path)
                .with_context(|| format!("parse source {}", path))?;
            let cache_path = if self.config.enabled {
                let cp = self.cache.cache_path_for(&name, &fp.sha_prefix);
                let nt = self.graph.serialize(NTRIPLES_FORMAT)?;
                CacheManager::atomic_write(&cp, &nt)?;
                self.cache.upsert(&name, path, &fp, &cp, count)?;
                cp
            } else {
                PathBuf::new()
            };
            (count, "source", cache_path)
        };

        // Record active entry.
        let mut active = self.active.lock().unwrap();
        *active = Some(ActiveEntry {
            name: name.clone(),
            source_path: path.to_string(),
            fingerprint: fp,
            cache_path: cache_path.clone(),
            auto_refresh: opts.auto_refresh,
            last_access: Mutex::new(Instant::now()),
            evicted: Mutex::new(false),
        });

        Ok(LoadResult {
            name,
            source_path: path.to_string(),
            triple_count,
            origin,
            cache_path: cache_path.to_string_lossy().into_owned(),
        })
    }

    /// Update `last_access` if there is an active entry.
    ///
    /// Calling `touch` on a registry with no active entry is a no-op — the
    /// registry stays in its empty state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use open_ontologies::graph::GraphStore;
    /// # use open_ontologies::state::StateDb;
    /// # use open_ontologies::registry::OntologyRegistry;
    /// # use open_ontologies::config::CacheConfig;
    /// # let tmp = tempfile::tempdir().unwrap();
    /// # let store = Arc::new(GraphStore::new());
    /// # let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// # let config = CacheConfig {
    /// #     dir: tmp.path().to_string_lossy().into_owned(),
    /// #     ..CacheConfig::default()
    /// # };
    /// # let reg = OntologyRegistry::new(store, db, config).unwrap();
    /// // No active entry — touch is a no-op, registry remains empty.
    /// reg.touch();
    /// let s = reg.status();
    /// assert!(s["active"].is_null());
    /// ```
    pub fn touch(&self) {
        if let Some(entry) = &*self.active.lock().unwrap() {
            *entry.last_access.lock().unwrap() = Instant::now();
        }
    }

    /// Make sure the in-memory store reflects the active entry.
    /// If the store was evicted, reload from cache (or source on refresh).
    /// No-op if no active entry exists.
    ///
    /// # Examples
    ///
    /// With no active entry, `ensure_loaded` succeeds and leaves the registry
    /// in an empty state:
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use open_ontologies::graph::GraphStore;
    /// # use open_ontologies::state::StateDb;
    /// # use open_ontologies::registry::OntologyRegistry;
    /// # use open_ontologies::config::CacheConfig;
    /// # let tmp = tempfile::tempdir().unwrap();
    /// # let store = Arc::new(GraphStore::new());
    /// # let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// # let config = CacheConfig {
    /// #     dir: tmp.path().to_string_lossy().into_owned(),
    /// #     ..CacheConfig::default()
    /// # };
    /// # let reg = OntologyRegistry::new(store, db, config).unwrap();
    /// reg.ensure_loaded().unwrap(); // no-op, no error
    /// assert!(reg.status()["active"].is_null());
    /// ```
    pub fn ensure_loaded(&self) -> Result<()> {
        // Single-flight guard.
        let _g = self.reload_lock.lock().unwrap();

        // Snapshot needed fields (avoid holding the active lock across reload).
        let needs_reload;
        let auto_refresh;
        let source_path;
        let stored_fp;
        let cache_path;
        let name;
        {
            let active_guard = self.active.lock().unwrap();
            let Some(entry) = active_guard.as_ref() else {
                return Ok(());
            };
            // Touch first.
            *entry.last_access.lock().unwrap() = Instant::now();
            needs_reload = *entry.evicted.lock().unwrap();
            auto_refresh = entry.auto_refresh;
            source_path = entry.source_path.clone();
            stored_fp = entry.fingerprint.clone();
            cache_path = entry.cache_path.clone();
            name = entry.name.clone();
        }

        // Auto-refresh: if the source file changed, recompile (even if not evicted).
        let mut refreshed = false;
        if auto_refresh && Path::new(&source_path).exists() {
            let cur = SourceFingerprint::from_path(Path::new(&source_path))?;
            if cur != stored_fp {
                // Source changed — re-parse and rewrite cache.
                self.graph.clear()?;
                let count = self.graph.load_file(&source_path)?;
                let new_cache = self.cache.cache_path_for(&name, &cur.sha_prefix);
                let nt = self.graph.serialize(NTRIPLES_FORMAT)?;
                CacheManager::atomic_write(&new_cache, &nt)?;
                self.cache.upsert(&name, &source_path, &cur, &new_cache, count)?;

                let mut active_guard = self.active.lock().unwrap();
                if let Some(entry) = active_guard.as_mut() {
                    entry.fingerprint = cur;
                    entry.cache_path = new_cache;
                    *entry.evicted.lock().unwrap() = false;
                }
                refreshed = true;
            }
        }

        if needs_reload && !refreshed {
            // Reload from N-Triples cache; fall back to source if cache file
            // is missing for some reason.
            if cache_path.exists() {
                let nt = std::fs::read_to_string(&cache_path)?;
                self.graph.clear()?;
                self.graph.load_ntriples(&nt)?;
            } else if Path::new(&source_path).exists() {
                self.graph.clear()?;
                self.graph.load_file(&source_path)?;
            } else {
                return Err(anyhow!(
                    "ontology '{}' was evicted but neither cache file '{}' nor source '{}' exists",
                    name, cache_path.display(), source_path
                ));
            }
            let active_guard = self.active.lock().unwrap();
            if let Some(entry) = active_guard.as_ref() {
                *entry.evicted.lock().unwrap() = false;
            }
            self.cache.touch(&name)?;
        }

        Ok(())
    }

    /// Evict the active entry if idle longer than `idle_ttl_secs`.
    /// Returns `true` when an eviction took place.
    ///
    /// When there is no active entry, or when the cache is disabled, or when
    /// `idle_ttl_secs` is 0, the tick is always a no-op.
    ///
    /// # Examples
    ///
    /// A registry with `idle_ttl_secs = 0` (default) never evicts:
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use open_ontologies::graph::GraphStore;
    /// # use open_ontologies::state::StateDb;
    /// # use open_ontologies::registry::OntologyRegistry;
    /// # use open_ontologies::config::CacheConfig;
    /// # let tmp = tempfile::tempdir().unwrap();
    /// # let store = Arc::new(GraphStore::new());
    /// # let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// # let config = CacheConfig {
    /// #     dir: tmp.path().to_string_lossy().into_owned(),
    /// #     idle_ttl_secs: 0,
    /// #     ..CacheConfig::default()
    /// # };
    /// # let reg = OntologyRegistry::new(store, db, config).unwrap();
    /// // idle_ttl_secs == 0 disables eviction entirely.
    /// assert_eq!(reg.evictor_tick().unwrap(), false);
    /// ```
    pub fn evictor_tick(&self) -> Result<bool> {
        if !self.config.enabled || self.config.idle_ttl_secs == 0 {
            return Ok(false);
        }
        let ttl = Duration::from_secs(self.config.idle_ttl_secs);
        let active = self.active.lock().unwrap();
        let Some(entry) = active.as_ref() else { return Ok(false) };
        let already = *entry.evicted.lock().unwrap();
        if already {
            return Ok(false);
        }
        let elapsed = entry.last_access.lock().unwrap().elapsed();
        if elapsed >= ttl {
            // Clear the in-memory store to release memory; cache file remains.
            self.graph.clear()?;
            *entry.evicted.lock().unwrap() = true;
            return Ok(true);
        }
        Ok(false)
    }

    /// Manually unload the active ontology (clear graph + drop active slot).
    /// The cache file is preserved unless `delete_cache` is true.
    ///
    /// Returns `None` when there was nothing to unload.
    ///
    /// # Examples
    ///
    /// Unloading a registry with no active entry returns `None`:
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use open_ontologies::graph::GraphStore;
    /// # use open_ontologies::state::StateDb;
    /// # use open_ontologies::registry::OntologyRegistry;
    /// # use open_ontologies::config::CacheConfig;
    /// # let tmp = tempfile::tempdir().unwrap();
    /// # let store = Arc::new(GraphStore::new());
    /// # let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// # let config = CacheConfig {
    /// #     dir: tmp.path().to_string_lossy().into_owned(),
    /// #     ..CacheConfig::default()
    /// # };
    /// # let reg = OntologyRegistry::new(store, db, config).unwrap();
    /// let result = reg.unload(false).unwrap();
    /// assert!(result.is_none());
    /// ```
    pub fn unload(&self, delete_cache: bool) -> Result<Option<String>> {
        let mut active = self.active.lock().unwrap();
        let Some(entry) = active.take() else { return Ok(None) };
        self.graph.clear()?;
        if delete_cache {
            self.cache.remove(&entry.name)?;
        }
        Ok(Some(entry.name))
    }

    /// Unload a specific named ontology.
    /// - If `name` matches the active slot, behaves like `unload(delete_cache)`.
    /// - If `name` is in the cache but not active, only the on-disk cache is
    ///   touched (since it was never in memory). With `delete_cache=true` the
    ///   cache file and DB row are removed; otherwise this is a no-op.
    ///
    /// Returns `Ok(true)` if anything was actually changed.
    pub fn unload_named(&self, name: &str, delete_cache: bool) -> Result<bool> {
        let active_name = self.active.lock().unwrap().as_ref().map(|e| e.name.clone());
        if active_name.as_deref() == Some(name) {
            return Ok(self.unload(delete_cache)?.is_some());
        }
        // Not active. The graph isn't holding it, so there's nothing to clear
        // in memory. Touch the cache only if requested.
        if delete_cache {
            if self.cache.get(name)?.is_none() {
                return Err(anyhow!("no cached ontology named '{}'", name));
            }
            self.cache.remove(name)?;
            return Ok(true);
        }
        // Verify the entry exists at least, so callers get a clear error
        // when they pass a typo.
        if self.cache.get(name)?.is_none() {
            return Err(anyhow!("no cached ontology named '{}'", name));
        }
        Ok(false)
    }

    /// Force recompile the active ontology from source (used by `onto_recompile`).
    pub fn recompile(&self) -> Result<LoadResult> {
        let (path, name, auto_refresh) = {
            let active = self.active.lock().unwrap();
            let entry = active
                .as_ref()
                .ok_or_else(|| anyhow!("no active ontology to recompile"))?;
            (entry.source_path.clone(), entry.name.clone(), entry.auto_refresh)
        };
        self.load_file(
            &path,
            LoadOptions {
                name: Some(name),
                auto_refresh,
                force_recompile: true,
            },
        )
    }

    /// Recompile a specific named ontology from its recorded source path.
    ///
    /// - If `name` is the active slot, this re-parses and replaces both the
    ///   in-memory store and the on-disk cache (same effect as `recompile()`).
    /// - If `name` is a non-active cache entry, the source is parsed into a
    ///   *temporary* `GraphStore`, the new N-Triples cache file is written
    ///   atomically, the metadata row is updated, and the active slot is
    ///   left completely untouched. This makes it safe to refresh background
    ///   ontologies without disturbing whatever is currently being queried.
    pub fn recompile_named(&self, name: &str) -> Result<LoadResult> {
        let active_name = self.active.lock().unwrap().as_ref().map(|e| e.name.clone());
        if active_name.as_deref() == Some(name) {
            return self.recompile();
        }
        let entry = self
            .cache
            .get(name)?
            .ok_or_else(|| anyhow!("no cached ontology named '{}'", name))?;
        let path = Path::new(&entry.source_path);
        if !path.exists() {
            return Err(anyhow!(
                "source file '{}' for cached ontology '{}' is missing",
                entry.source_path, name
            ));
        }
        // Parse into an isolated graph so we don't disturb the active slot.
        let isolated_graph = GraphStore::new();
        let count = isolated_graph
            .load_file(&entry.source_path)
            .with_context(|| format!("parse source {}", entry.source_path))?;
        let fp = SourceFingerprint::from_path(path)?;
        let cache_path = self.cache.cache_path_for(name, &fp.sha_prefix);
        let nt = isolated_graph.serialize(NTRIPLES_FORMAT)?;
        CacheManager::atomic_write(&cache_path, &nt)?;
        // If the sha-prefix changed, the new cache_path differs from the old
        // one. Remove the old file to avoid leaking stale .nt files.
        if entry.cache_path != cache_path.to_string_lossy() {
            let _ = std::fs::remove_file(&entry.cache_path);
        }
        self.cache.upsert(name, &entry.source_path, &fp, &cache_path, count)?;
        Ok(LoadResult {
            name: name.to_string(),
            source_path: entry.source_path,
            triple_count: count,
            origin: "source",
            cache_path: cache_path.to_string_lossy().into_owned(),
        })
    }

    /// Return all cached ontologies, with extra runtime flags
    /// (`is_active`, `in_memory`) so callers can present a single rich list.
    ///
    /// On a freshly constructed registry the list is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use open_ontologies::graph::GraphStore;
    /// # use open_ontologies::state::StateDb;
    /// # use open_ontologies::registry::OntologyRegistry;
    /// # use open_ontologies::config::CacheConfig;
    /// # let tmp = tempfile::tempdir().unwrap();
    /// # let store = Arc::new(GraphStore::new());
    /// # let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// # let config = CacheConfig {
    /// #     dir: tmp.path().to_string_lossy().into_owned(),
    /// #     ..CacheConfig::default()
    /// # };
    /// # let reg = OntologyRegistry::new(store, db, config).unwrap();
    /// let entries = reg.list_cached().unwrap();
    /// assert!(entries.is_empty());
    /// ```
    pub fn list_cached(&self) -> Result<Vec<serde_json::Value>> {
        let active_guard = self.active.lock().unwrap();
        let active_name = active_guard.as_ref().map(|e| e.name.clone());
        let evicted = active_guard
            .as_ref()
            .map(|e| *e.evicted.lock().unwrap())
            .unwrap_or(false);
        drop(active_guard);

        let entries = self.cache.list()?;
        let out = entries
            .into_iter()
            .map(|e| {
                let is_active = active_name.as_deref() == Some(e.name.as_str());
                let in_memory = is_active && !evicted;
                serde_json::json!({
                    "name": e.name,
                    "source_path": e.source_path,
                    "cache_path": e.cache_path,
                    TRIPLE_COUNT_KEY: e.triple_count,
                    "source_mtime": e.source_mtime,
                    "source_size": e.source_size,
                    "compiled_at": e.compiled_at,
                    "last_access_at": e.last_access_at,
                    "is_active": is_active,
                    "in_memory": in_memory,
                })
            })
            .collect();
        Ok(out)
    }

    /// Status snapshot for `onto_cache_status`.
    ///
    /// On a freshly constructed registry with no loaded ontology the `"active"`
    /// key is `null` and `"cache_entries"` is an empty array.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use open_ontologies::graph::GraphStore;
    /// # use open_ontologies::state::StateDb;
    /// # use open_ontologies::registry::OntologyRegistry;
    /// # use open_ontologies::config::CacheConfig;
    /// # let tmp = tempfile::tempdir().unwrap();
    /// # let store = Arc::new(GraphStore::new());
    /// # let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// # let config = CacheConfig {
    /// #     dir: tmp.path().to_string_lossy().into_owned(),
    /// #     ..CacheConfig::default()
    /// # };
    /// # let reg = OntologyRegistry::new(store, db, config).unwrap();
    /// let s = reg.status();
    /// assert!(s["active"].is_null());
    /// assert_eq!(s["cache_entries"].as_array().unwrap().len(), 0);
    /// ```
    pub fn status(&self) -> serde_json::Value {
        let active = self.active.lock().unwrap();
        let active_json = if let Some(entry) = active.as_ref() {
            let evicted = *entry.evicted.lock().unwrap();
            let last_access_secs = entry.last_access.lock().unwrap().elapsed().as_secs();
            serde_json::json!({
                "name": entry.name,
                "source_path": entry.source_path,
                "cache_path": entry.cache_path.to_string_lossy(),
                "auto_refresh": entry.auto_refresh,
                "evicted": evicted,
                "idle_seconds": last_access_secs,
                "in_memory_triples": self.graph.triple_count(),
            })
        } else {
            serde_json::Value::Null
        };
        let entries: Vec<_> = self
            .cache
            .list()
            .unwrap_or_default()
            .into_iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name,
                    "source_path": e.source_path,
                    "cache_path": e.cache_path,
                    TRIPLE_COUNT_KEY: e.triple_count,
                    "compiled_at": e.compiled_at,
                    "last_access_at": e.last_access_at,
                })
            })
            .collect();
        serde_json::json!({
            "active": active_json,
            "cache_entries": entries,
            "config": {
                "enabled": self.config.enabled,
                "dir": self.config.dir,
                "idle_ttl_secs": self.config.idle_ttl_secs,
                "auto_refresh": self.config.auto_refresh,
            }
        })
    }
}

/// Spawn the background evictor task. Returns a `JoinHandle` that callers can
/// keep alive; dropping it does NOT abort (the task is detached but will exit
/// with the runtime).
///
/// # Examples
///
/// ```no_run
/// # use std::sync::Arc;
/// # use open_ontologies::graph::GraphStore;
/// # use open_ontologies::state::StateDb;
/// # use open_ontologies::registry::{OntologyRegistry, spawn_evictor};
/// # use open_ontologies::config::CacheConfig;
/// # #[tokio::main]
/// # async fn main() {
/// #     let tmp = tempfile::tempdir().unwrap();
/// #     let store = Arc::new(GraphStore::new());
/// #     let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
/// #     let config = CacheConfig {
/// #         dir: tmp.path().to_string_lossy().into_owned(),
/// #         ..CacheConfig::default()
/// #     };
/// #     let reg = Arc::new(OntologyRegistry::new(store, db, config).unwrap());
/// let handle = spawn_evictor(Arc::clone(&reg));
/// // Keep `handle` alive for the duration of the server.
/// drop(handle);
/// # }
/// ```
pub fn spawn_evictor(registry: Arc<OntologyRegistry>) -> tokio::task::JoinHandle<()> {
    let interval_secs = registry.config().evictor_interval_secs.max(1);
    tokio::spawn(async move {
        let mut ticker =
            tokio::time::interval(Duration::from_secs(interval_secs));
        // Skip the immediate tick so we don't evict before any access happens.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            if let Err(e) = registry.evictor_tick() {
                tracing::warn!("ontology evictor tick failed: {}", e);
            }
        }
    })
}
