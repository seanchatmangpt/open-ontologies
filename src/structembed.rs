//! Learn Poincaré embeddings from the ontology class hierarchy.
//! Uses Riemannian SGD to push parent-child pairs closer and
//! negative samples apart in the Poincaré ball.
//!
//! # Overview
//!
//! [`StructuralTrainer`] derives positional embeddings that encode the
//! *topology* of an ontology hierarchy.  After training, classes closer to
//! the origin are more general (nearer the root), while leaf classes sit
//! further from the origin.  The embeddings complement text-based embeddings
//! and are used as the structural channel in [`crate::vecstore::VecStore`].
//!
//! # Quick-start
//!
//! ```no_run
//! // Requires `embeddings` feature
//! use open_ontologies::structembed::StructuralTrainer;
//! use open_ontologies::graph::GraphStore;
//!
//! let store = GraphStore::new().unwrap();
//! // load_str / load_file would populate `store` with OWL classes here.
//!
//! let trainer = StructuralTrainer::new(
//!     16,   // embedding dimension
//!     20,   // training epochs
//!     0.01, // initial learning rate
//! );
//! let embeddings = trainer.train(&store).unwrap();
//! // `embeddings` maps class IRI → Vec<f32> of length 16.
//! for (iri, vec) in &embeddings {
//!     assert_eq!(vec.len(), 16, "wrong dim for {iri}");
//! }
//! ```

use crate::graph::GraphStore;
use crate::poincare::{poincare_distance, project_to_ball, rsgd_step};
use anyhow::Result;
use std::collections::HashMap;
use std::collections::HashSet;

/// Trainer that learns Poincaré embeddings from the ontology class hierarchy.
///
/// Each class is assigned an embedding in the Poincaré ball. After training,
/// parent classes lie closer to the origin than their children, and semantically
/// unrelated classes are pushed apart via negative sampling.
///
/// # Example
///
/// ```no_run
/// // Requires `embeddings` feature
/// use open_ontologies::structembed::StructuralTrainer;
///
/// // Create a trainer with 64-dimensional embeddings, 50 epochs, learning rate 0.01
/// let trainer = StructuralTrainer::new(64, 50, 0.01);
/// ```
pub struct StructuralTrainer {
    dim: usize,
    epochs: usize,
    lr: f32,
}

impl StructuralTrainer {
    /// Creates a new [`StructuralTrainer`] with the given hyperparameters.
    ///
    /// # Parameters
    ///
    /// - `dim` — Dimensionality of the Poincaré ball embedding space.
    /// - `epochs` — Number of training passes over the hierarchy edges.
    /// - `lr` — Initial learning rate (decays linearly to 0 over training).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Requires `embeddings` feature
    /// use open_ontologies::structembed::StructuralTrainer;
    ///
    /// // Minimal trainer: 2-D embeddings, 1 epoch, large learning rate
    /// let small = StructuralTrainer::new(2, 1, 0.1);
    ///
    /// // Production-scale trainer
    /// let prod = StructuralTrainer::new(128, 200, 0.005);
    /// ```
    pub fn new(dim: usize, epochs: usize, lr: f32) -> Self {
        Self { dim, epochs, lr }
    }

    /// Extract (parent, child) edges from the graph via SPARQL.
    fn extract_edges(store: &GraphStore) -> Vec<(String, String)> {
        let query = r#"
            SELECT DISTINCT ?child ?parent WHERE {
                ?child <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?parent .
                ?child a <http://www.w3.org/2002/07/owl#Class> .
                ?parent a <http://www.w3.org/2002/07/owl#Class> .
                FILTER(isIRI(?child) && isIRI(?parent))
            }
        "#;

        let result = match store.sparql_select(query) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        let parsed: serde_json::Value = match serde_json::from_str(&result) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        parsed["results"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|row| {
                let child = row["child"].as_str()?.trim_matches(|c| c == '<' || c == '>').to_string();
                let parent = row["parent"].as_str()?.trim_matches(|c| c == '<' || c == '>').to_string();
                Some((parent, child))
            })
            .collect()
    }

    /// Extract all class IRIs from the graph.
    fn extract_all_classes(store: &GraphStore) -> Vec<String> {
        let query = r#"
            SELECT DISTINCT ?class WHERE {
                ?class a <http://www.w3.org/2002/07/owl#Class> .
                FILTER(isIRI(?class))
            }
        "#;

        let result = match store.sparql_select(query) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        let parsed: serde_json::Value = match serde_json::from_str(&result) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        parsed["results"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|row| {
                Some(row["class"].as_str()?.trim_matches(|c| c == '<' || c == '>').to_string())
            })
            .collect()
    }

    /// Train Poincaré embeddings from the ontology hierarchy.
    ///
    /// Queries the `store` for `owl:Class` nodes and `rdfs:subClassOf` edges,
    /// then runs Riemannian SGD to place classes in the Poincaré ball.
    /// Returns a map from class IRI to its embedding vector.
    ///
    /// Returns an empty map when the store contains no `owl:Class` individuals.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Requires `embeddings` feature
    /// use open_ontologies::structembed::StructuralTrainer;
    /// use open_ontologies::graph::GraphStore;
    ///
    /// // An empty store produces an empty embedding map.
    /// let store = GraphStore::new().unwrap();
    /// let trainer = StructuralTrainer::new(8, 5, 0.01);
    /// let embeddings = trainer.train(&store).unwrap();
    /// assert!(embeddings.is_empty());
    /// ```
    ///
    /// ```no_run
    /// // Requires `embeddings` feature
    /// use open_ontologies::structembed::StructuralTrainer;
    /// use open_ontologies::graph::GraphStore;
    ///
    /// // After loading an ontology the map contains one entry per class.
    /// let store = GraphStore::new().unwrap();
    /// // store.load_str(SOME_TURTLE, "text/turtle").unwrap();
    /// let trainer = StructuralTrainer::new(4, 10, 0.01);
    /// let embeddings = trainer.train(&store).unwrap();
    /// // Each embedding has exactly `dim` components.
    /// for (_iri, vec) in &embeddings {
    ///     assert_eq!(vec.len(), 4);
    /// }
    /// ```
    pub fn train(&self, store: &GraphStore) -> Result<HashMap<String, Vec<f32>>> {
        let edges = Self::extract_edges(store);
        let classes = Self::extract_all_classes(store);

        if classes.is_empty() {
            return Ok(HashMap::new());
        }

        // Initialize embeddings near origin
        let mut embeddings: HashMap<String, Vec<f32>> = HashMap::new();
        for (i, class) in classes.iter().enumerate() {
            let init: Vec<f32> = (0..self.dim)
                .map(|j| {
                    let seed = (i * self.dim + j) as f32;
                    ((seed * 2654435761.0) % 1000.0) / 50000.0 - 0.01
                })
                .collect();
            embeddings.insert(class.clone(), project_to_ball(&init, 1e-5));
        }

        if edges.is_empty() {
            return Ok(embeddings);
        }

        // Identify root nodes (parents that are never children)
        let children: HashSet<&str> = edges.iter().map(|(_, c)| c.as_str()).collect();
        let roots: Vec<String> = edges.iter()
            .map(|(p, _)| p.clone())
            .filter(|p| !children.contains(p.as_str()))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let num_classes = classes.len();
        for epoch in 0..self.epochs {
            let lr = self.lr * (1.0 - epoch as f32 / self.epochs as f32);

            // Pull parent-child pairs closer
            for (edge_i, (parent, child)) in edges.iter().enumerate() {
                let parent_emb = embeddings[parent].clone();
                let child_emb = embeddings[child].clone();

                let dist = poincare_distance(&parent_emb, &child_emb);
                if dist > 0.0 {
                    // Gradient to pull them closer
                    let grad_parent: Vec<f32> = parent_emb.iter().zip(child_emb.iter())
                        .map(|(p, c)| p - c)
                        .collect();
                    let grad_child: Vec<f32> = child_emb.iter().zip(parent_emb.iter())
                        .map(|(c, p)| c - p)
                        .collect();

                    let new_parent = rsgd_step(&parent_emb, &grad_parent, lr);
                    let new_child = rsgd_step(&child_emb, &grad_child, lr);
                    embeddings.insert(parent.clone(), new_parent);
                    embeddings.insert(child.clone(), new_child);
                }

                // Push child outward (further from origin than parent)
                let parent_emb = embeddings[parent].clone();
                let child_emb = embeddings[child].clone();
                let parent_norm: f32 = parent_emb.iter().map(|x| x * x).sum::<f32>().sqrt();
                let child_norm: f32 = child_emb.iter().map(|x| x * x).sum::<f32>().sqrt();

                if child_norm <= parent_norm + 0.01 {
                    // Push child away from origin along its direction
                    let grad_outward: Vec<f32> = child_emb.iter()
                        .map(|c| -c)
                        .collect();
                    let new_child = rsgd_step(&child_emb, &grad_outward, lr * 0.5);
                    embeddings.insert(child.clone(), new_child);
                }

                // Negative sampling: vary index by both epoch and edge
                let neg_idx = (epoch * 7 + edge_i * 13 + 3) % num_classes;
                let neg_iri = &classes[neg_idx];
                if neg_iri != parent && neg_iri != child {
                    let neg_emb = embeddings[neg_iri].clone();
                    let child_emb = embeddings[child].clone();

                    let neg_dist = poincare_distance(&child_emb, &neg_emb);
                    let margin = 1.0;
                    if neg_dist < margin {
                        let grad_neg: Vec<f32> = neg_emb.iter().zip(child_emb.iter())
                            .map(|(n, c)| c - n)
                            .collect();
                        let new_neg = rsgd_step(&neg_emb, &grad_neg, lr);
                        embeddings.insert(neg_iri.clone(), new_neg);
                    }
                }
            }

            // Pull root nodes toward origin
            for root in &roots {
                let root_emb = embeddings[root].clone();
                // Gradient: point toward origin (i.e., the embedding itself, pulling it back)
                let new_root = rsgd_step(&root_emb, &root_emb, lr * 0.5);
                embeddings.insert(root.clone(), new_root);
            }
        }

        Ok(embeddings)
    }
}
