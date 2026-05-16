use crate::drift::jaro_winkler;
use crate::graph::GraphStore;

/// A single crosswalk mapping row.
///
/// # Examples
///
/// ```
/// use open_ontologies::clinical::CrosswalkRow;
///
/// let row = CrosswalkRow {
///     source_code: "J18.9".to_string(),
///     source_system: "ICD-10".to_string(),
///     target_code: "233604007".to_string(),
///     target_system: "SNOMED".to_string(),
///     relation: "exactMatch".to_string(),
///     source_label: "Pneumonia, unspecified organism".to_string(),
///     target_label: "Pneumonia".to_string(),
/// };
///
/// assert_eq!(row.source_system, "ICD-10");
/// assert_eq!(row.target_system, "SNOMED");
/// ```
#[derive(Debug, Clone)]
pub struct CrosswalkRow {
    pub source_code: String,
    pub source_system: String,
    pub target_code: String,
    pub target_system: String,
    pub relation: String,
    pub source_label: String,
    pub target_label: String,
}

/// Clinical crosswalks backed by a Parquet file.
pub struct ClinicalCrosswalks {
    rows: Vec<CrosswalkRow>,
}

impl ClinicalCrosswalks {
    /// Load crosswalk data from a Parquet file.
    ///
    /// Returns an error if the file does not exist or cannot be read as Parquet.
    ///
    /// ```no_run
    /// use open_ontologies::clinical::ClinicalCrosswalks;
    ///
    /// // Parquet file must exist on disk — download from your data pipeline.
    /// let cw = ClinicalCrosswalks::load("crosswalks.parquet")
    ///     .expect("parquet file must exist");
    /// println!("loaded {} rows", cw.lookup("J18.9", "ICD-10").len());
    /// ```
    pub fn load(path: &str) -> anyhow::Result<Self> {
        use arrow::array::StringArray;
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        use std::fs::File;

        let file = File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;

        let mut rows = Vec::new();
        for batch in reader {
            let batch = batch?;
            let source_code = batch.column_by_name("source_code")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let source_system = batch.column_by_name("source_system")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let target_code = batch.column_by_name("target_code")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let target_system = batch.column_by_name("target_system")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let relation = batch.column_by_name("relation")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let source_label = batch.column_by_name("source_label")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let target_label = batch.column_by_name("target_label")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>());

            if let (Some(sc), Some(ss), Some(tc), Some(ts), Some(rel), Some(sl), Some(tl)) =
                (source_code, source_system, target_code, target_system, relation, source_label, target_label)
            {
                for i in 0..batch.num_rows() {
                    rows.push(CrosswalkRow {
                        source_code: sc.value(i).to_string(),
                        source_system: ss.value(i).to_string(),
                        target_code: tc.value(i).to_string(),
                        target_system: ts.value(i).to_string(),
                        relation: rel.value(i).to_string(),
                        source_label: sl.value(i).to_string(),
                        target_label: tl.value(i).to_string(),
                    });
                }
            }
        }

        Ok(Self { rows })
    }

    /// Build a `ClinicalCrosswalks` directly from a `Vec<CrosswalkRow>`.
    ///
    /// Useful in tests and doctests where no Parquet file is needed.
    ///
    /// ```
    /// use open_ontologies::clinical::{ClinicalCrosswalks, CrosswalkRow};
    ///
    /// let rows = vec![CrosswalkRow {
    ///     source_code: "E11".to_string(),
    ///     source_system: "ICD-10".to_string(),
    ///     target_code: "44054006".to_string(),
    ///     target_system: "SNOMED".to_string(),
    ///     relation: "exactMatch".to_string(),
    ///     source_label: "Type 2 diabetes mellitus".to_string(),
    ///     target_label: "Diabetes mellitus type 2".to_string(),
    /// }];
    ///
    /// let cw = ClinicalCrosswalks::from_rows(rows);
    /// assert_eq!(cw.lookup("E11", "ICD-10").len(), 1);
    /// ```
    pub fn from_rows(rows: Vec<CrosswalkRow>) -> Self {
        Self { rows }
    }

    /// Look up all mappings for a given code and system.
    ///
    /// Returns an empty slice when no mapping exists for the code/system pair.
    ///
    /// ```
    /// use open_ontologies::clinical::{ClinicalCrosswalks, CrosswalkRow};
    ///
    /// let cw = ClinicalCrosswalks::from_rows(vec![
    ///     CrosswalkRow {
    ///         source_code: "J18.9".to_string(),
    ///         source_system: "ICD-10".to_string(),
    ///         target_code: "233604007".to_string(),
    ///         target_system: "SNOMED".to_string(),
    ///         relation: "exactMatch".to_string(),
    ///         source_label: "Pneumonia, unspecified organism".to_string(),
    ///         target_label: "Pneumonia".to_string(),
    ///     },
    /// ]);
    ///
    /// // Known code → one result
    /// let hits = cw.lookup("J18.9", "ICD-10");
    /// assert_eq!(hits.len(), 1);
    /// assert_eq!(hits[0].target_system, "SNOMED");
    ///
    /// // Wrong system → no result
    /// assert!(cw.lookup("J18.9", "SNOMED").is_empty());
    ///
    /// // Unknown code → no result
    /// assert!(cw.lookup("Z99.99", "ICD-10").is_empty());
    /// ```
    pub fn lookup(&self, code: &str, system: &str) -> Vec<&CrosswalkRow> {
        self.rows
            .iter()
            .filter(|r| r.source_code == code && r.source_system == system)
            .collect()
    }

    /// Fuzzy search across all labels using Jaro-Winkler.
    ///
    /// Only results with similarity > 0.6 are returned, capped at 20, sorted
    /// by descending similarity.
    ///
    /// ```
    /// use open_ontologies::clinical::{ClinicalCrosswalks, CrosswalkRow};
    ///
    /// let cw = ClinicalCrosswalks::from_rows(vec![
    ///     CrosswalkRow {
    ///         source_code: "J18.9".to_string(),
    ///         source_system: "ICD-10".to_string(),
    ///         target_code: "233604007".to_string(),
    ///         target_system: "SNOMED".to_string(),
    ///         relation: "exactMatch".to_string(),
    ///         source_label: "Pneumonia, unspecified organism".to_string(),
    ///         target_label: "Pneumonia".to_string(),
    ///     },
    /// ]);
    ///
    /// // Near-exact label match returns a result
    /// let results = cw.search_label("Pneumonia");
    /// assert!(!results.is_empty());
    /// let first = &results[0];
    /// assert!(first["similarity"].as_f64().unwrap() > 0.6);
    /// assert_eq!(first["source_system"].as_str().unwrap(), "ICD-10");
    ///
    /// // Completely unrelated text → no result
    /// let none = cw.search_label("xyzzy_gibberish_99");
    /// assert!(none.is_empty());
    /// ```
    pub fn search_label(&self, text: &str) -> Vec<serde_json::Value> {
        let text_lower = text.to_lowercase();
        let mut results: Vec<(f64, &CrosswalkRow)> = self
            .rows
            .iter()
            .map(|r| {
                let sl_score = jaro_winkler(&text_lower, &r.source_label.to_lowercase());
                let tl_score = jaro_winkler(&text_lower, &r.target_label.to_lowercase());
                (sl_score.max(tl_score), r)
            })
            .filter(|(score, _)| *score > 0.6)
            .collect();

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(20);

        results
            .iter()
            .map(|(score, r)| {
                serde_json::json!({
                    "source_code": r.source_code,
                    "source_system": r.source_system,
                    "target_code": r.target_code,
                    "target_system": r.target_system,
                    "source_label": r.source_label,
                    "target_label": r.target_label,
                    "similarity": score,
                })
            })
            .collect()
    }

    /// Validate clinical terms in a loaded ontology against the crosswalk data.
    ///
    /// Queries the graph for all `owl:Class` labels and fuzzy-matches each label
    /// against the crosswalk table. Returns a JSON object with `validated`,
    /// `unmatched`, and `total_classes` counts.
    ///
    /// ```
    /// use open_ontologies::clinical::{ClinicalCrosswalks, CrosswalkRow};
    /// use open_ontologies::graph::GraphStore;
    ///
    /// let graph = GraphStore::new();
    /// graph.load_turtle(
    ///     "@prefix owl: <http://www.w3.org/2002/07/owl#> .\
    ///      @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\
    ///      <urn:ex:Pneumonia> a owl:Class ; rdfs:label \"Pneumonia\" .",
    ///     None,
    /// ).unwrap();
    ///
    /// let cw = ClinicalCrosswalks::from_rows(vec![CrosswalkRow {
    ///     source_code: "J18.9".to_string(),
    ///     source_system: "ICD-10".to_string(),
    ///     target_code: "233604007".to_string(),
    ///     target_system: "SNOMED".to_string(),
    ///     relation: "exactMatch".to_string(),
    ///     source_label: "Pneumonia, unspecified organism".to_string(),
    ///     target_label: "Pneumonia".to_string(),
    /// }]);
    ///
    /// let report = cw.validate_clinical(&graph);
    /// let parsed: serde_json::Value = serde_json::from_str(&report).unwrap();
    ///
    /// // One class was loaded; it matched "Pneumonia" in the crosswalk.
    /// assert_eq!(parsed["total_classes"].as_u64().unwrap(), 1);
    /// assert_eq!(parsed["validated"].as_array().unwrap().len(), 1);
    /// assert_eq!(parsed["unmatched"].as_array().unwrap().len(), 0);
    /// ```
    pub fn validate_clinical(&self, graph: &GraphStore) -> String {
        let label_query = "SELECT ?c ?l WHERE { \
            ?c a <http://www.w3.org/2002/07/owl#Class> . \
            ?c <http://www.w3.org/2000/01/rdf-schema#label> ?l \
        }";

        let mut validated = Vec::new();
        let mut unmatched = Vec::new();

        if let Ok(json) = graph.sparql_select(label_query)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json)
            && let Some(results) = parsed["results"].as_array()
        {
            for row in results {
                if let (Some(class_iri), Some(label)) =
                    (row["c"].as_str(), row["l"].as_str())
                {
                    let label_clean = label
                        .trim_matches('"')
                        .split("^^")
                        .next()
                        .unwrap_or("")
                        .trim_matches('"');
                    let matches = self.search_label(label_clean);
                    if !matches.is_empty() {
                        validated.push(serde_json::json!({
                            "class": class_iri,
                            "label": label_clean,
                            "matches": matches.len(),
                            "best_match": matches[0],
                        }));
                    } else {
                        unmatched.push(serde_json::json!({
                            "class": class_iri,
                            "label": label_clean,
                        }));
                    }
                }
            }
        }

        serde_json::json!({
            "validated": validated,
            "unmatched": unmatched,
            "total_classes": validated.len() + unmatched.len(),
        })
        .to_string()
    }

    /// Enrich an ontology class with a SKOS mapping triple.
    ///
    /// Looks up `code`/`system` in the crosswalk table and inserts a
    /// `skos:exactMatch` triple for the first mapping found.  Returns
    /// `{"ok": false, "error": "...", "hint": "..."}` when no mapping exists
    /// or when the SPARQL update fails.
    ///
    /// ```
    /// use open_ontologies::clinical::{ClinicalCrosswalks, CrosswalkRow};
    /// use open_ontologies::graph::GraphStore;
    ///
    /// let graph = GraphStore::new();
    ///
    /// // --- missing code returns a structured error ---
    /// let cw = ClinicalCrosswalks::from_rows(vec![]);
    /// let result = cw.enrich(&graph, "urn:ex:MyClass", "UNKNOWN", "ICD-10");
    /// let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    /// assert_eq!(parsed["ok"].as_bool().unwrap(), false);
    /// assert!(parsed["error"].as_str().unwrap().contains("No crosswalk"));
    /// assert!(parsed["hint"].as_str().is_some());
    ///
    /// // --- valid code inserts skos:exactMatch ---
    /// let cw2 = ClinicalCrosswalks::from_rows(vec![CrosswalkRow {
    ///     source_code: "J18.9".to_string(),
    ///     source_system: "ICD-10".to_string(),
    ///     target_code: "233604007".to_string(),
    ///     target_system: "SNOMED".to_string(),
    ///     relation: "exactMatch".to_string(),
    ///     source_label: "Pneumonia, unspecified organism".to_string(),
    ///     target_label: "Pneumonia".to_string(),
    /// }]);
    /// let result2 = cw2.enrich(&graph, "urn:ex:Pneumonia", "J18.9", "ICD-10");
    /// let parsed2: serde_json::Value = serde_json::from_str(&result2).unwrap();
    /// assert_eq!(parsed2["ok"].as_bool().unwrap(), true);
    /// assert_eq!(parsed2["enriched"].as_str().unwrap(), "urn:ex:Pneumonia");
    /// ```
    pub fn enrich(&self, graph: &GraphStore, class_iri: &str, code: &str, system: &str) -> String {
        let mappings = self.lookup(code, system);
        if mappings.is_empty() {
            return serde_json::json!({
                "ok": false,
                "error": format!("No crosswalk found for {} in {}", code, system),
                "hint": "Check that the code and system are present in the crosswalk table. \
                         Use onto_crosswalk to browse available mappings.",
            })
            .to_string();
        }

        let target = &mappings[0];
        let update = format!(
            "INSERT DATA {{ <{}> <http://www.w3.org/2004/02/skos/core#exactMatch> <urn:{}:{}> . \
             <urn:{}:{}> <http://www.w3.org/2000/01/rdf-schema#label> \"{}\" . }}",
            class_iri,
            target.target_system, target.target_code,
            target.target_system, target.target_code,
            target.target_label,
        );

        match graph.sparql_update(&update) {
            Ok(n) => serde_json::json!({
                "ok": true,
                "enriched": class_iri,
                "mapped_to": format!("{}:{}", target.target_system, target.target_code),
                "triples_added": n,
            })
            .to_string(),
            Err(e) => serde_json::json!({
                "ok": false,
                "error": e.to_string(),
                "hint": "The SPARQL update failed. Verify the graph store is writable \
                         and the class IRI is a valid URI.",
            })
            .to_string(),
        }
    }
}
