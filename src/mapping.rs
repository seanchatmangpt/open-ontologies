use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single field-to-predicate mapping within a `MappingConfig`.
///
/// # Examples
///
/// ```
/// use open_ontologies::mapping::FieldMapping;
///
/// // Construct a typed literal field mapping
/// let fm = FieldMapping {
///     field: "name".to_string(),
///     predicate: "https://schema.org/name".to_string(),
///     datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
///     class: None,
///     lookup: false,
/// };
/// assert_eq!(fm.field, "name");
/// assert_eq!(fm.predicate, "https://schema.org/name");
/// assert!(fm.datatype.is_some());
/// assert!(!fm.lookup);
/// ```
///
/// A lookup field produces an IRI object rather than a literal:
///
/// ```
/// use open_ontologies::mapping::FieldMapping;
///
/// let lookup = FieldMapping {
///     field: "category_id".to_string(),
///     predicate: "https://schema.org/category".to_string(),
///     datatype: None,
///     class: Some("https://schema.org/Category".to_string()),
///     lookup: true,
/// };
/// assert!(lookup.lookup);
/// assert_eq!(lookup.class.as_deref(), Some("https://schema.org/Category"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub field: String,
    pub predicate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datatype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(default)]
    pub lookup: bool,
}

/// Configuration that describes how structured data rows map to RDF triples.
///
/// # Examples
///
/// ```
/// use open_ontologies::mapping::{FieldMapping, MappingConfig};
///
/// // Directly construct a MappingConfig with two field entries
/// let cfg = MappingConfig {
///     base_iri: "https://example.org/data/".to_string(),
///     id_field: "id".to_string(),
///     class: "https://example.org/schema#Person".to_string(),
///     mappings: vec![
///         FieldMapping {
///             field: "id".to_string(),
///             predicate: "https://example.org/data/ont#id".to_string(),
///             datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
///             class: None,
///             lookup: false,
///         },
///         FieldMapping {
///             field: "name".to_string(),
///             predicate: "https://example.org/data/ont#name".to_string(),
///             datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
///             class: None,
///             lookup: false,
///         },
///     ],
/// };
/// assert_eq!(cfg.id_field, "id");
/// assert_eq!(cfg.mappings.len(), 2);
/// assert_eq!(cfg.class, "https://example.org/schema#Person");
///
/// // find_by_source_field returns the first mapping with the given field name
/// let found = cfg.find_by_source_field("name");
/// assert!(found.is_some());
/// assert_eq!(found.unwrap().predicate, "https://example.org/data/ont#name");
///
/// // Returns None for an unknown field
/// assert!(cfg.find_by_source_field("unknown").is_none());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingConfig {
    pub base_iri: String,
    pub id_field: String,
    pub class: String,
    pub mappings: Vec<FieldMapping>,
}

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

impl MappingConfig {
    /// Return the first [`FieldMapping`] whose `field` name matches `source_field`.
    ///
    /// This is a pure O(n) scan; `None` is returned when no mapping matches.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::mapping::MappingConfig;
    ///
    /// let headers: Vec<String> = vec!["id".into(), "name".into(), "email".into()];
    /// let cfg = MappingConfig::from_headers(
    ///     &headers,
    ///     "https://example.org/",
    ///     "https://example.org/schema#Contact",
    /// );
    ///
    /// // Hit: "name" is in the mapping
    /// let m = cfg.find_by_source_field("name").expect("name must be mapped");
    /// assert_eq!(m.field, "name");
    /// assert!(m.predicate.contains("name"));
    ///
    /// // Miss: "phone" was never declared
    /// assert!(cfg.find_by_source_field("phone").is_none());
    /// ```
    pub fn find_by_source_field(&self, source_field: &str) -> Option<&FieldMapping> {
        self.mappings.iter().find(|m| m.field == source_field)
    }

    /// Generate a naive 1:1 mapping from column headers.
    ///
    /// The first column becomes the `id_field`. Every column gets an
    /// `xsd:string` predicate under `{base_iri}ont#`.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::mapping::MappingConfig;
    ///
    /// let headers: Vec<String> = vec!["id".into(), "name".into(), "age".into()];
    /// let cfg = MappingConfig::from_headers(
    ///     &headers,
    ///     "https://example.org/data/",
    ///     "https://example.org/schema#Person",
    /// );
    ///
    /// assert_eq!(cfg.id_field, "id");
    /// assert_eq!(cfg.mappings.len(), 3);
    /// assert_eq!(cfg.mappings[1].predicate, "https://example.org/data/ont#name");
    /// assert_eq!(cfg.class, "https://example.org/schema#Person");
    /// ```
    ///
    /// An empty header slice produces a default `id_field` of `"id"` and no mappings:
    ///
    /// ```
    /// use open_ontologies::mapping::MappingConfig;
    ///
    /// let cfg = MappingConfig::from_headers(&[], "https://example.org/", "https://example.org/schema#Thing");
    /// assert_eq!(cfg.id_field, "id");
    /// assert!(cfg.mappings.is_empty());
    /// ```
    pub fn from_headers(headers: &[String], base_iri: &str, class: &str) -> Self {
        let id_field = headers
            .first()
            .cloned()
            .unwrap_or_else(|| "id".to_string());

        let mappings = headers
            .iter()
            .map(|h| FieldMapping {
                field: h.clone(),
                predicate: format!("{}ont#{}", base_iri, sanitize_iri(h)),
                datatype: Some(XSD_STRING.to_string()),
                class: None,
                lookup: false,
            })
            .collect();

        Self {
            base_iri: base_iri.to_string(),
            id_field,
            class: class.to_string(),
            mappings,
        }
    }

    /// Convert a single data row to a list of N-Triples lines.
    ///
    /// Produces:
    /// - An `rdf:type` triple linking the subject to `self.class`
    /// - One triple per mapped field that has a non-empty value in the row
    ///
    /// Lookup fields produce IRI objects; typed fields produce typed literals;
    /// everything else produces plain literals.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use open_ontologies::mapping::MappingConfig;
    ///
    /// let headers: Vec<String> = vec!["id".into(), "name".into()];
    /// let cfg = MappingConfig::from_headers(
    ///     &headers,
    ///     "https://example.org/data/",
    ///     "https://example.org/schema#Person",
    /// );
    ///
    /// let mut row = HashMap::new();
    /// row.insert("id".into(), "alice".into());
    /// row.insert("name".into(), "Alice".into());
    ///
    /// let triples = cfg.row_to_triples(&row);
    ///
    /// // First triple is always rdf:type
    /// assert!(triples[0].contains("<https://example.org/data/alice>"));
    /// assert!(triples[0].contains("rdf-syntax-ns#type"));
    /// assert!(triples[0].contains("<https://example.org/schema#Person>"));
    ///
    /// // Second triple is the name field
    /// assert!(triples.iter().any(|t| t.contains("\"Alice\"")));
    /// ```
    ///
    /// Empty values are skipped; only the `rdf:type` triple is emitted:
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use open_ontologies::mapping::MappingConfig;
    ///
    /// let headers: Vec<String> = vec!["id".into(), "name".into()];
    /// let cfg = MappingConfig::from_headers(
    ///     &headers,
    ///     "https://example.org/data/",
    ///     "https://example.org/schema#Person",
    /// );
    ///
    /// let mut row = HashMap::new();
    /// row.insert("id".into(), "bob".into());
    /// // "name" is absent — should be skipped
    ///
    /// let triples = cfg.row_to_triples(&row);
    /// assert_eq!(triples.len(), 2); // rdf:type + id field only
    /// ```
    pub fn row_to_triples(&self, row: &HashMap<String, String>) -> Vec<String> {
        let subject_id = row
            .get(&self.id_field)
            .filter(|v| !v.is_empty())
            .map(|v| sanitize_iri(v))
            .unwrap_or_else(rand_id);

        let subject = format!("<{}{}>", self.base_iri, subject_id);
        let mut triples = Vec::new();

        // rdf:type triple
        triples.push(format!("{} <{}> <{}> .", subject, RDF_TYPE, self.class));

        for mapping in &self.mappings {
            let value = match row.get(&mapping.field) {
                Some(v) if !v.is_empty() => v,
                _ => continue,
            };

            let object = if mapping.lookup {
                // Lookup field: produce an IRI object
                format!("<{}{}>", self.base_iri, sanitize_iri(value))
            } else if let Some(ref dt) = mapping.datatype {
                // Typed literal
                format!("\"{}\"^^<{}>", escape_ntriples(value), dt)
            } else {
                // Plain literal
                format!("\"{}\"", escape_ntriples(value))
            };

            triples.push(format!("{} <{}> {} .", subject, mapping.predicate, object));
        }

        triples
    }

    /// Convert multiple rows to a single N-Triples string.
    ///
    /// Triples from each row are joined with newlines. The result is a valid
    /// N-Triples document when non-empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use open_ontologies::mapping::MappingConfig;
    ///
    /// let headers: Vec<String> = vec!["id".into(), "label".into()];
    /// let cfg = MappingConfig::from_headers(
    ///     &headers,
    ///     "https://example.org/data/",
    ///     "https://example.org/schema#Item",
    /// );
    ///
    /// let mut row1 = HashMap::new();
    /// row1.insert("id".into(), "item1".into());
    /// row1.insert("label".into(), "First".into());
    ///
    /// let mut row2 = HashMap::new();
    /// row2.insert("id".into(), "item2".into());
    /// row2.insert("label".into(), "Second".into());
    ///
    /// let ntriples = cfg.rows_to_ntriples(&[row1, row2]);
    ///
    /// assert!(ntriples.contains("item1"));
    /// assert!(ntriples.contains("item2"));
    /// assert!(ntriples.contains("\"First\""));
    /// assert!(ntriples.contains("\"Second\""));
    /// // Lines are newline-separated
    /// assert!(ntriples.contains('\n'));
    /// ```
    ///
    /// An empty slice produces an empty string:
    ///
    /// ```
    /// use open_ontologies::mapping::MappingConfig;
    ///
    /// let headers: Vec<String> = vec!["id".into()];
    /// let cfg = MappingConfig::from_headers(&headers, "https://example.org/", "https://example.org/schema#T");
    /// assert_eq!(cfg.rows_to_ntriples(&[]), "");
    /// ```
    pub fn rows_to_ntriples(&self, rows: &[HashMap<String, String>]) -> String {
        rows.iter()
            .flat_map(|row| self.row_to_triples(row))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Replace spaces and special characters that are invalid in IRIs with underscores.
fn sanitize_iri(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' | '<' | '>' | '{' | '}' | '|' | '\\' | '^' | '`' | '?' => '_',
            _ => c,
        })
        .collect()
}

/// Escape characters that are special in N-Triples string literals.
fn escape_ntriples(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '\\' => vec!['\\', '\\'],
            '"' => vec!['\\', '"'],
            '\n' => vec!['\\', 'n'],
            '\r' => vec!['\\', 'r'],
            '\t' => vec!['\\', 't'],
            _ => vec![c],
        })
        .collect()
}

/// Generate a simple random ID based on the current time's sub-second nanos.
fn rand_id() -> String {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("_auto_{}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_iri() {
        assert_eq!(sanitize_iri("hello world"), "hello_world");
        assert_eq!(sanitize_iri("a<b>c"), "a_b_c");
        assert_eq!(sanitize_iri("no_change"), "no_change");
        assert_eq!(sanitize_iri("PEAK_OF_PERFECTION_?"), "PEAK_OF_PERFECTION__");
    }

    #[test]
    fn test_escape_ntriples() {
        assert_eq!(escape_ntriples(r#"say "hi""#), r#"say \"hi\""#);
        assert_eq!(escape_ntriples("line\nnew"), "line\\nnew");
    }

    #[test]
    fn test_rand_id_format() {
        let id = rand_id();
        assert!(id.starts_with("_auto_"));
    }
}
