//! In-memory vector store with dual-space search (cosine + Poincaré)
//! and SQLite persistence.

use crate::poincare::{cosine_similarity, l2_normalize, poincare_distance};
use crate::state::StateDb;
use std::collections::HashMap;

#[derive(Clone)]
struct VecEntry {
    text_vec: Vec<f32>,
    struct_vec: Vec<f32>,
}

/// Brute-force dual-space vector store.
pub struct VecStore {
    db: StateDb,
    entries: HashMap<String, VecEntry>,
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
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::vecstore::VecStore;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = VecStore::new(db);
    /// assert!(store.is_empty());
    /// assert_eq!(store.len(), 0);
    /// ```
    pub fn new(db: StateDb) -> Self {
        Self {
            db,
            entries: HashMap::new(),
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
    /// ```
    pub fn upsert(&mut self, iri: &str, text_vec: &[f32], struct_vec: &[f32]) {
        self.entries.insert(iri.to_string(), VecEntry {
            text_vec: l2_normalize(text_vec),
            struct_vec: struct_vec.to_vec(),
        });
    }

    /// Removes the entry for `iri` if it exists.
    ///
    /// Removing a non-existent IRI is a no-op.
    ///
    /// # Examples
    ///
    /// ```
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
    /// ```
    pub fn remove(&mut self, iri: &str) {
        self.entries.remove(iri);
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

    /// Returns the top-`k` entries ranked by Poincaré distance to `query`.
    ///
    /// Results are returned in ascending distance order (nearest first).
    ///
    /// # Examples
    ///
    /// ```
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
    /// ```
    pub fn search_poincare(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scores: Vec<(String, f32)> = self.entries.iter()
            .map(|(iri, e)| (iri.clone(), poincare_distance(query, &e.struct_vec)))
            .collect();
        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
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

    pub fn load_from_db(&mut self) -> anyhow::Result<()> {
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
        Ok(())
    }

    /// Returns the number of entries currently held in the store.
    ///
    /// # Examples
    ///
    /// ```
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
    /// ```
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when the store contains no entries.
    ///
    /// # Examples
    ///
    /// ```
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
