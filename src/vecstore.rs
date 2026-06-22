//! In-memory vector store with dual-space search (cosine + Poincaré)
//! and SQLite persistence.
//!
//! # Overview
//!
//! [`VecStore`] holds text embeddings (L2-normalised for cosine search) and
//! structural embeddings (Poincaré ball coordinates for hyperbolic search)
//! keyed by class IRI.  The three search modes are:
//!
//! - [`VecStore::search_cosine`] — standard inner-product ranking on text vecs
//! - [`VecStore::search_poincare`] — hyperbolic distance ranking on struct vecs
//! - [`VecStore::search_product`] — weighted combination of both spaces
//!
//! # Quick-start
//!
//! ```no_run
//! // Requires `embeddings` feature
//! use open_ontologies::state::StateDb;
//! use open_ontologies::vecstore::VecStore;
//! use std::path::Path;
//!
//! let db = StateDb::open(Path::new(":memory:")).unwrap();
//! let mut store = VecStore::new(db);
//!
//! store.upsert("urn:ex:Dog",  &[1.0, 0.0, 0.0], &[0.1, 0.0, 0.0]);
//! store.upsert("urn:ex:Cat",  &[0.9, 0.1, 0.0], &[0.15, 0.0, 0.0]);
//! store.upsert("urn:ex:Fish", &[0.0, 0.0, 1.0], &[0.5, 0.0, 0.0]);
//!
//! // Text search: query most similar to Dog
//! let hits = store.search_cosine(&[1.0, 0.0, 0.0], 2);
//! assert_eq!(hits[0].0, "urn:ex:Dog");
//! assert_eq!(hits.len(), 2);
//! ```

use crate::hnsw_index::{CosineIndex, PoincareIndex};
use crate::poincare::{cosine_similarity, l2_normalize, poincare_distance};
use crate::state::StateDb;
use std::collections::HashMap;

#[derive(Clone)]
struct VecEntry {
    text_vec: Vec<f32>,
    struct_vec: Vec<f32>,
}

/// Brute-force dual-space vector store with an opt-in HNSW cosine index.
pub struct VecStore {
    db: StateDb,
    entries: HashMap<String, VecEntry>,
    /// Lazily-built HNSW index over `text_vec`s for accelerated cosine
    /// search. Invalidated on every mutation; rebuilt on first
    /// `search_cosine_hnsw` after a mutation. The existing
    /// `search_cosine` linear scan is unchanged and continues to work
    /// without HNSW.
    cosine_index: Option<CosineIndex>,
    /// Lazily-built HNSW index over `struct_vec`s for accelerated Poincaré
    /// search. Same invalidation semantics as `cosine_index`. The existing
    /// brute-force `search_poincare` is unchanged.
    poincare_index: Option<PoincareIndex>,
}

impl VecStore {
    /// Creates a new, empty `VecStore` backed by the given [`StateDb`].
    ///
    /// The store starts with no entries; use [`upsert`](VecStore::upsert) to
    /// populate it before calling any search method.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "embeddings")]
    /// # {
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = VecStore::new(db);
    /// assert!(store.is_empty());
    /// assert_eq!(store.len(), 0);
    /// # }
    /// ```
    pub fn new(db: StateDb) -> Self {
        Self {
            db,
            entries: HashMap::new(),
            cosine_index: None,
            poincare_index: None,
        }
    }

    /// Inserts or replaces an entry for `iri`.
    ///
    /// The `text_vec` is L2-normalised internally before storage; `struct_vec`
    /// is stored as-is for Poincaré distance computation.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "embeddings")]
    /// # {
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    ///
    /// store.upsert("urn:ex:A", &[1.0, 0.0], &[0.1, 0.2]);
    /// assert_eq!(store.len(), 1);
    ///
    /// // Upserting the same IRI replaces the existing entry, not appends.
    /// store.upsert("urn:ex:A", &[0.0, 1.0], &[0.3, 0.4]);
    /// assert_eq!(store.len(), 1);
    /// # }
    /// ```
    pub fn upsert(&mut self, iri: &str, text_vec: &[f32], struct_vec: &[f32]) {
        self.entries.insert(iri.to_string(), VecEntry {
            text_vec: l2_normalize(text_vec),
            struct_vec: struct_vec.to_vec(),
        });
        // Invalidate BOTH HNSW indices — instant-distance is immutable.
        self.cosine_index = None;
        self.poincare_index = None;
    }

    /// Removes the entry for `iri` if it exists.
    ///
    /// Removing a non-existent IRI is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "embeddings")]
    /// # {
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    ///
    /// store.upsert("urn:ex:B", &[1.0, 0.0], &[0.0, 0.0]);
    /// assert_eq!(store.len(), 1);
    ///
    /// store.remove("urn:ex:B");
    /// assert!(store.is_empty());
    ///
    /// // Removing again is a no-op.
    /// store.remove("urn:ex:B");
    /// assert!(store.is_empty());
    /// # }
    /// ```
    pub fn remove(&mut self, iri: &str) {
        self.entries.remove(iri);
        self.cosine_index = None;
        self.poincare_index = None;
    }

    /// Returns the top-`k` entries ranked by cosine similarity to `query`.
    ///
    /// Results are returned in descending similarity order (most similar
    /// first). If the store has fewer than `top_k` entries the returned
    /// `Vec` is shorter than `top_k`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "embeddings")]
    /// # {
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    ///
    /// // Insert two entries whose text vectors point in different directions.
    /// store.upsert("urn:ex:X", &[1.0, 0.0], &[0.0, 0.0]);
    /// store.upsert("urn:ex:Y", &[0.0, 1.0], &[0.0, 0.0]);
    ///
    /// // A query aligned with X should rank X first.
    /// let results = store.search_cosine(&[1.0, 0.0], 2);
    /// assert_eq!(results.len(), 2);
    /// assert_eq!(results[0].0, "urn:ex:X");
    /// assert!(results[0].1 > results[1].1);
    /// # }
    /// ```
    ///
    /// ```
    /// # #[cfg(feature = "embeddings")]
    /// # {
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    ///
    /// store.upsert("urn:ex:Z", &[1.0, 0.0], &[0.0, 0.0]);
    ///
    /// // Requesting more results than entries returns only what is available.
    /// let results = store.search_cosine(&[1.0, 0.0], 100);
    /// assert_eq!(results.len(), 1);
    ///
    /// // An empty store returns an empty result.
    /// let empty_db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let empty = VecStore::new(empty_db);
    /// assert_eq!(empty.search_cosine(&[1.0, 0.0], 5).len(), 0);
    /// # }
    /// ```
    pub fn search_cosine(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let query_norm = l2_normalize(query);
        let mut scores: Vec<(String, f32)> = self.entries.iter()
            .map(|(iri, e)| (iri.clone(), cosine_similarity(&query_norm, &e.text_vec)))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    /// HNSW-accelerated cosine search. Approximate top-k via the HNSW index;
    /// builds the index lazily on first call (and after any mutation).
    ///
    /// Same query/output semantics as [`Self::search_cosine`] (results sorted
    /// by cosine similarity descending, top_k truncation, same scale), but
    /// sub-linear query time once the index is warm. The trade-off vs the
    /// exact brute-force scan: approximate top-k under default HNSW params,
    /// rebuild cost on every mutation.
    ///
    /// Use this when:
    /// - The store has more than a few hundred entries
    /// - You expect many queries between mutations (`embed-once,
    ///   search-many-times`)
    /// - Approximate top-k is acceptable
    ///
    /// Otherwise stick with [`Self::search_cosine`].
    pub fn search_cosine_hnsw(&mut self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        if self.entries.is_empty() {
            return Vec::new();
        }
        if self.cosine_index.is_none() {
            // Lazy build from current entries. Vectors are already L2-normalised
            // (the upsert path guarantees that), so the HNSW index sees unit
            // vectors and the cosine distance == 1 - dot product.
            let points: Vec<(String, Vec<f32>)> = self
                .entries
                .iter()
                .map(|(iri, e)| (iri.clone(), e.text_vec.clone()))
                .collect();
            self.cosine_index = Some(CosineIndex::build(points));
        }
        let query_norm = l2_normalize(query);
        match self.cosine_index.as_mut() {
            Some(idx) => idx.search(&query_norm, top_k),
            None => Vec::new(),
        }
    }


    /// Returns the top-`k` entries ranked by Poincaré distance to `query`.
    ///
    /// Results are returned in ascending distance order (nearest first).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "embeddings")]
    /// # {
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    ///
    /// // Struct vectors must lie strictly inside the Poincaré ball (norm < 1).
    /// store.upsert("urn:ex:P", &[1.0, 0.0], &[0.1, 0.0]);
    /// store.upsert("urn:ex:Q", &[0.0, 1.0], &[0.4, 0.0]);
    ///
    /// // A query at the origin is closest to the entry nearer the origin.
    /// let results = store.search_poincare(&[0.0, 0.0], 2);
    /// assert_eq!(results.len(), 2);
    /// // Nearest entry (smallest distance) is first.
    /// assert!(results[0].1 <= results[1].1);
    /// # }
    /// ```
    pub fn search_poincare(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scores: Vec<(String, f32)> = self.entries.iter()
            .map(|(iri, e)| (iri.clone(), poincare_distance(query, &e.struct_vec)))
            .collect();
        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    /// HNSW-accelerated Poincaré search. Mirrors [`Self::search_cosine_hnsw`]
    /// but over the structural-embedding space (`struct_vec`) with hyperbolic
    /// distance. Builds the Poincaré index lazily on first call; rebuilds on
    /// any mutation.
    pub fn search_poincare_hnsw(&mut self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        if self.entries.is_empty() {
            return Vec::new();
        }
        if self.poincare_index.is_none() {
            let points: Vec<(String, Vec<f32>)> = self
                .entries
                .iter()
                .map(|(iri, e)| (iri.clone(), e.struct_vec.clone()))
                .collect();
            self.poincare_index = Some(PoincareIndex::build(points));
        }
        match self.poincare_index.as_mut() {
            Some(idx) => idx.search(query, top_k),
            None => Vec::new(),
        }
    }


    /// Returns the top-`k` entries ranked by a linear combination of cosine
    /// similarity and Poincaré proximity.
    ///
    /// The combined score is `alpha * cosine + (1 - alpha) * (1 / (1 + poincaré))`.
    /// Set `alpha = 1.0` for pure cosine; `alpha = 0.0` for pure Poincaré proximity.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "embeddings")]
    /// # {
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    ///
    /// store.upsert("urn:ex:M", &[1.0, 0.0], &[0.1, 0.0]);
    /// store.upsert("urn:ex:N", &[0.0, 1.0], &[0.4, 0.0]);
    ///
    /// let results = store.search_product(&[1.0, 0.0], &[0.0, 0.0], 2, 0.5);
    /// assert_eq!(results.len(), 2);
    /// // Results are in descending combined-score order.
    /// assert!(results[0].1 >= results[1].1);
    /// # }
    /// ```
    ///
    /// ```
    /// # #[cfg(feature = "embeddings")]
    /// # {
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    ///
    /// store.upsert("urn:ex:O", &[1.0, 0.0], &[0.1, 0.0]);
    /// store.upsert("urn:ex:P2", &[0.0, 1.0], &[0.4, 0.0]);
    ///
    /// // alpha=1.0 is identical to search_cosine: only text similarity matters.
    /// let product = store.search_product(&[1.0, 0.0], &[0.0, 0.0], 2, 1.0);
    /// let cosine  = store.search_cosine(&[1.0, 0.0], 2);
    /// assert_eq!(product[0].0, cosine[0].0);
    /// # }
    /// ```
    pub fn search_product(
        &self,
        text_query: &[f32],
        struct_query: &[f32],
        top_k: usize,
        alpha: f32,
    ) -> Vec<(String, f32)> {
        let text_norm = l2_normalize(text_query);
        let mut scores: Vec<(String, f32)> = self.entries.iter()
            .map(|(iri, e)| {
                let cos = cosine_similarity(&text_norm, &e.text_vec);
                let poinc = poincare_distance(struct_query, &e.struct_vec);
                let poinc_sim = 1.0 / (1.0 + poinc);
                let combined = alpha * cos + (1.0 - alpha) * poinc_sim;
                (iri.clone(), combined)
            })
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    /// Deterministic FNV-1a 64-bit fingerprint of the entry set. Stable across
    /// processes; used to detect when a cached HNSW index is stale because the
    /// underlying vectors have changed. Includes both keys and text-vec bytes
    /// in the hash so re-embedding the same IRI with a new vector triggers a
    /// rebuild.
    fn entries_fingerprint(&self) -> Vec<u8> {
        let mut keys: Vec<&String> = self.entries.keys().collect();
        keys.sort();
        let mut hash: u64 = 0xcbf29ce484222325;
        for k in keys {
            for byte in k.as_bytes() {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
            let v = &self.entries[k];
            for f in &v.text_vec {
                for byte in f.to_le_bytes() {
                    hash ^= byte as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                }
            }
        }
        hash.to_le_bytes().to_vec()
    }

    /// Force-rebuild the HNSW cosine index using explicit HNSW parameters.
    /// Drops any previously-built index. The new index is held in memory; call
    /// [`Self::persist_cosine_index`] to save it.
    pub fn rebuild_cosine_index(&mut self, params: crate::hnsw_index::BuildParams) {
        if self.entries.is_empty() {
            self.cosine_index = None;
            return;
        }
        let points: Vec<(String, Vec<f32>)> = self
            .entries
            .iter()
            .map(|(iri, e)| (iri.clone(), e.text_vec.clone()))
            .collect();
        self.cosine_index = Some(crate::hnsw_index::CosineIndex::build_with_params(
            points, params,
        ));
    }

    /// Force-rebuild the HNSW Poincaré index using explicit HNSW parameters.
    /// Same semantics as [`Self::rebuild_cosine_index`] but for the
    /// structural-embedding space.
    pub fn rebuild_poincare_index(&mut self, params: crate::hnsw_index::BuildParams) {
        if self.entries.is_empty() {
            self.poincare_index = None;
            return;
        }
        let points: Vec<(String, Vec<f32>)> = self
            .entries
            .iter()
            .map(|(iri, e)| (iri.clone(), e.struct_vec.clone()))
            .collect();
        self.poincare_index = Some(crate::hnsw_index::PoincareIndex::build_with_params(
            points, params,
        ));
    }

    /// Persist the current HNSW cosine index to SQLite (table `hnsw_index_cache`).
    /// Builds the index first if it isn't built. Subsequent `load_cosine_index()`
    /// calls (e.g. at process startup via `load_from_db`) read it back and skip
    /// the rebuild as long as the entry fingerprint matches.
    pub fn persist_cosine_index(&mut self) -> anyhow::Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }
        if self.cosine_index.is_none() {
            let points: Vec<(String, Vec<f32>)> = self
                .entries
                .iter()
                .map(|(iri, e)| (iri.clone(), e.text_vec.clone()))
                .collect();
            self.cosine_index = Some(CosineIndex::build(points));
        }
        let bytes = match self.cosine_index.as_ref() {
            Some(idx) => idx.to_bytes()?,
            None => return Ok(()),
        };
        let fp = self.entries_fingerprint();
        let count = self.entries.len() as i64;
        let conn = self.db.conn();
        conn.execute(
            "INSERT OR REPLACE INTO hnsw_index_cache (kind, entries_hash, entry_count, serialised) \
             VALUES ('cosine', ?1, ?2, ?3)",
            rusqlite::params![fp, count, bytes],
        )?;
        Ok(())
    }

    /// Try to load a previously-persisted HNSW cosine index. If the stored
    /// fingerprint matches the current entries' fingerprint, the index is
    /// deserialised in-place and subsequent `search_cosine_hnsw` calls skip
    /// the rebuild. If the fingerprint mismatches (or no cache exists), this
    /// is a no-op and the next `search_cosine_hnsw` rebuilds normally.
    pub fn load_cosine_index(&mut self) -> anyhow::Result<bool> {
        let conn = self.db.conn();
        let row: Option<(Vec<u8>, Vec<u8>)> = conn
            .query_row(
                "SELECT entries_hash, serialised FROM hnsw_index_cache WHERE kind = 'cosine'",
                [],
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .ok();
        let (stored_hash, bytes) = match row {
            Some(x) => x,
            None => return Ok(false),
        };
        let current_hash = self.entries_fingerprint();
        if stored_hash != current_hash {
            // Stale — let the rebuild path handle it next time.
            return Ok(false);
        }
        self.cosine_index = Some(CosineIndex::from_bytes(&bytes)?);
        Ok(true)
    }

    /// Async background flush of the cosine index. Serialises the index
    /// synchronously (in-memory bincode work, typically < 100ms for ontologies
    /// under ~10k classes), then dispatches the SQLite write to a tokio
    /// `spawn_blocking` task. Returns a JoinHandle so the caller can await
    /// completion if they care; otherwise fire-and-forget is fine.
    ///
    /// Use when persisting from inside an async MCP tool handler over a
    /// large index, where the SQLite write latency would otherwise hold up
    /// the handler thread. For small indices the sync `persist_cosine_index`
    /// is just as fast.
    pub fn persist_cosine_index_async(
        &mut self,
    ) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<()>>> {
        if self.entries.is_empty() {
            return Ok(tokio::task::spawn(async { Ok::<(), anyhow::Error>(()) }));
        }
        if self.cosine_index.is_none() {
            let points: Vec<(String, Vec<f32>)> = self
                .entries
                .iter()
                .map(|(iri, e)| (iri.clone(), e.text_vec.clone()))
                .collect();
            self.cosine_index = Some(CosineIndex::build(points));
        }
        let bytes = self
            .cosine_index
            .as_ref()
            .expect("cosine index just built or pre-existing")
            .to_bytes()?;
        let fp = self.entries_fingerprint();
        let count = self.entries.len() as i64;
        let db = self.db.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let conn = db.conn();
            conn.execute(
                "INSERT OR REPLACE INTO hnsw_index_cache (kind, entries_hash, entry_count, serialised) \
                 VALUES ('cosine', ?1, ?2, ?3)",
                rusqlite::params![fp, count, bytes],
            )?;
            Ok::<(), anyhow::Error>(())
        });
        Ok(handle)
    }

    /// Async background flush of the Poincaré index. See
    /// [`Self::persist_cosine_index_async`] for semantics.
    pub fn persist_poincare_index_async(
        &mut self,
    ) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<()>>> {
        if self.entries.is_empty() {
            return Ok(tokio::task::spawn(async { Ok::<(), anyhow::Error>(()) }));
        }
        if self.poincare_index.is_none() {
            let points: Vec<(String, Vec<f32>)> = self
                .entries
                .iter()
                .map(|(iri, e)| (iri.clone(), e.struct_vec.clone()))
                .collect();
            self.poincare_index = Some(PoincareIndex::build(points));
        }
        let bytes = self
            .poincare_index
            .as_ref()
            .expect("poincare index just built or pre-existing")
            .to_bytes()?;
        let fp = self.entries_fingerprint();
        let count = self.entries.len() as i64;
        let db = self.db.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let conn = db.conn();
            conn.execute(
                "INSERT OR REPLACE INTO hnsw_index_cache (kind, entries_hash, entry_count, serialised) \
                 VALUES ('poincare', ?1, ?2, ?3)",
                rusqlite::params![fp, count, bytes],
            )?;
            Ok::<(), anyhow::Error>(())
        });
        Ok(handle)
    }

    /// Persist the Poincaré index. Mirrors [`Self::persist_cosine_index`] but
    /// uses `kind = 'poincare'` in the cache row. Both indices use the SAME
    /// entries fingerprint (the entry set is identical; only the index over
    /// it differs) so a single fingerprint mismatch invalidates both kinds.
    pub fn persist_poincare_index(&mut self) -> anyhow::Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }
        if self.poincare_index.is_none() {
            let points: Vec<(String, Vec<f32>)> = self
                .entries
                .iter()
                .map(|(iri, e)| (iri.clone(), e.struct_vec.clone()))
                .collect();
            self.poincare_index = Some(PoincareIndex::build(points));
        }
        let bytes = match self.poincare_index.as_ref() {
            Some(idx) => idx.to_bytes()?,
            None => return Ok(()),
        };
        let fp = self.entries_fingerprint();
        let count = self.entries.len() as i64;
        let conn = self.db.conn();
        conn.execute(
            "INSERT OR REPLACE INTO hnsw_index_cache (kind, entries_hash, entry_count, serialised) \
             VALUES ('poincare', ?1, ?2, ?3)",
            rusqlite::params![fp, count, bytes],
        )?;
        Ok(())
    }

    /// Try to load a persisted Poincaré index. Same fingerprint-validation as
    /// [`Self::load_cosine_index`].
    pub fn load_poincare_index(&mut self) -> anyhow::Result<bool> {
        let conn = self.db.conn();
        let row: Option<(Vec<u8>, Vec<u8>)> = conn
            .query_row(
                "SELECT entries_hash, serialised FROM hnsw_index_cache WHERE kind = 'poincare'",
                [],
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .ok();
        let (stored_hash, bytes) = match row {
            Some(x) => x,
            None => return Ok(false),
        };
        let current_hash = self.entries_fingerprint();
        if stored_hash != current_hash {
            return Ok(false);
        }
        self.poincare_index = Some(PoincareIndex::from_bytes(&bytes)?);
        Ok(true)
    }


    /// Persists all in-memory entries to the backing SQLite database.
    ///
    /// The operation runs inside a transaction: the `embeddings` table is
    /// cleared first, then all current entries are inserted.  On error the
    /// transaction is rolled back and no partial state is written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Requires `embeddings` feature
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    ///
    /// store.upsert("urn:ex:H", &[1.0, 0.0], &[0.1, 0.2]);
    /// store.upsert("urn:ex:I", &[0.0, 1.0], &[0.3, 0.4]);
    ///
    /// // Persist to SQLite.
    /// store.persist().unwrap();
    ///
    /// // In-memory state is unchanged after persisting.
    /// assert_eq!(store.len(), 2);
    /// ```
    pub fn persist(&self) -> anyhow::Result<()> {
        let conn = self.db.conn();
        let tx = conn.unchecked_transaction()?;
        tx.execute("DELETE FROM embeddings", [])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO embeddings (iri, text_vec, struct_vec, text_dim, struct_dim) VALUES (?1, ?2, ?3, ?4, ?5)"
            )?;
            for (iri, entry) in &self.entries {
                let text_bytes = f32_slice_to_bytes(&entry.text_vec);
                let struct_bytes = f32_slice_to_bytes(&entry.struct_vec);
                stmt.execute(rusqlite::params![
                    iri,
                    text_bytes,
                    struct_bytes,
                    entry.text_vec.len() as i64,
                    entry.struct_vec.len() as i64,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Loads all entries from the backing SQLite database into memory,
    /// replacing any entries already in the store.
    ///
    /// Complements [`persist`](VecStore::persist): call `persist` to save,
    /// then reconstruct a store via `new` + `load_from_db` to restore.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Requires `embeddings` feature
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// // Build and persist a store.
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    /// store.upsert("urn:ex:J", &[1.0, 0.0], &[0.1, 0.0]);
    /// store.upsert("urn:ex:K", &[0.0, 1.0], &[0.2, 0.0]);
    /// store.persist().unwrap();
    ///
    /// // `load_from_db` populates an empty store from SQLite.
    /// store.remove("urn:ex:J");
    /// store.remove("urn:ex:K");
    /// assert!(store.is_empty());
    /// store.load_from_db().unwrap();
    /// assert_eq!(store.len(), 2);
    /// ```
    pub fn load_from_db(&mut self) -> anyhow::Result<()> {
        // Scope the connection + statement so the conn MutexGuard is dropped
        // before we call `load_cosine_index` (which re-acquires it).
        {
            let conn = self.db.conn();
            let mut stmt = conn.prepare("SELECT iri, text_vec, struct_vec FROM embeddings")?;
            let rows = stmt.query_map([], |row| {
                let iri: String = row.get(0)?;
                let text_bytes: Vec<u8> = row.get(1)?;
                let struct_bytes: Vec<u8> = row.get(2)?;
                Ok((iri, text_bytes, struct_bytes))
            })?;

            for row in rows {
                let (iri, text_bytes, struct_bytes) = row?;
                self.entries.insert(iri, VecEntry {
                    text_vec: bytes_to_f32_vec(&text_bytes),
                    struct_vec: bytes_to_f32_vec(&struct_bytes),
                });
            }
        }
        // Invalidate any previously-built HNSW indices; try to load persisted
        // ones. If the persisted fingerprint matches the entries we just loaded,
        // the next `search_cosine_hnsw` / `search_poincare_hnsw` skips rebuild.
        self.cosine_index = None;
        self.poincare_index = None;
        let _ = self.load_cosine_index()?;
        let _ = self.load_poincare_index()?;
        Ok(())
    }

    /// Returns the number of entries currently held in the store.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "embeddings")]
    /// # {
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    /// assert_eq!(store.len(), 0);
    ///
    /// store.upsert("urn:ex:C", &[1.0, 0.0], &[0.0, 0.0]);
    /// assert_eq!(store.len(), 1);
    ///
    /// store.upsert("urn:ex:D", &[0.0, 1.0], &[0.0, 0.0]);
    /// assert_eq!(store.len(), 2);
    /// # }
    /// ```
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when the store contains no entries.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "embeddings")]
    /// # {
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    /// assert!(store.is_empty());
    ///
    /// store.upsert("urn:ex:E", &[1.0, 0.0], &[0.0, 0.0]);
    /// assert!(!store.is_empty());
    /// # }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the (L2-normalised) text embedding stored for `iri`, or `None`
    /// if the IRI has not been inserted.
    ///
    /// Note: the returned slice reflects the *normalised* vector, which may
    /// differ from the raw `text_vec` passed to [`upsert`](VecStore::upsert).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "embeddings")]
    /// # {
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    ///
    /// assert!(store.get_text_vec("urn:ex:F").is_none());
    ///
    /// store.upsert("urn:ex:F", &[3.0, 4.0], &[0.0, 0.0]);
    /// let v = store.get_text_vec("urn:ex:F").unwrap();
    ///
    /// // The vector is L2-normalised: ‖v‖ ≈ 1.
    /// let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    /// assert!((norm - 1.0).abs() < 1e-5);
    /// # }
    /// ```
    pub fn get_text_vec(&self, iri: &str) -> Option<&[f32]> {
        self.entries.get(iri).map(|e| e.text_vec.as_slice())
    }

    /// Returns the structural embedding stored for `iri`, or `None` if the
    /// IRI has not been inserted.
    ///
    /// Unlike the text vector, the structural vector is stored exactly as
    /// supplied to [`upsert`](VecStore::upsert) (no normalisation).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "embeddings")]
    /// # {
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let mut store = VecStore::new(db);
    ///
    /// assert!(store.get_struct_vec("urn:ex:G").is_none());
    ///
    /// store.upsert("urn:ex:G", &[1.0, 0.0], &[0.2, 0.3]);
    /// let sv = store.get_struct_vec("urn:ex:G").unwrap();
    /// assert_eq!(sv, &[0.2_f32, 0.3_f32]);
    /// # }
    /// ```
    pub fn get_struct_vec(&self, iri: &str) -> Option<&[f32]> {
        self.entries.get(iri).map(|e| e.struct_vec.as_slice())
    }
}

fn f32_slice_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn bytes_to_f32_vec(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}
