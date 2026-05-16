use std::io::Cursor;
use std::sync::Mutex;

use oxigraph::io::{RdfFormat, RdfParser, RdfSerializer};
use oxigraph::model::*;
use oxigraph::sparql::QueryResults;
use oxigraph::store::Store;

// ── Format token constants ────────────────────────────────────────────────────

/// Canonical format token for Turtle RDF.
///
/// # Examples
///
/// ```
/// use open_ontologies::graph::GRAPH_FORMAT_TURTLE;
/// assert_eq!(GRAPH_FORMAT_TURTLE, "turtle");
/// ```
pub const GRAPH_FORMAT_TURTLE: &str = "turtle";

/// Canonical format token for N-Triples RDF.
///
/// # Examples
///
/// ```
/// use open_ontologies::graph::GRAPH_FORMAT_NTRIPLES;
/// assert_eq!(GRAPH_FORMAT_NTRIPLES, "ntriples");
/// ```
pub const GRAPH_FORMAT_NTRIPLES: &str = "ntriples";

/// Canonical format token for RDF/XML.
///
/// # Examples
///
/// ```
/// use open_ontologies::graph::GRAPH_FORMAT_RDFXML;
/// assert_eq!(GRAPH_FORMAT_RDFXML, "rdfxml");
/// ```
pub const GRAPH_FORMAT_RDFXML: &str = "rdfxml";

/// Canonical format token for N-Quads RDF.
///
/// # Examples
///
/// ```
/// use open_ontologies::graph::GRAPH_FORMAT_NQUADS;
/// assert_eq!(GRAPH_FORMAT_NQUADS, "nquads");
/// ```
pub const GRAPH_FORMAT_NQUADS: &str = "nquads";

/// Canonical format token for TriG RDF.
///
/// # Examples
///
/// ```
/// use open_ontologies::graph::GRAPH_FORMAT_TRIG;
/// assert_eq!(GRAPH_FORMAT_TRIG, "trig");
/// ```
pub const GRAPH_FORMAT_TRIG: &str = "trig";

// ── Stats JSON key constants ──────────────────────────────────────────────────

/// JSON key for the triple count in `stats()` and SPARQL CONSTRUCT results.
///
/// # Examples
///
/// ```
/// use open_ontologies::graph::GRAPH_STAT_TRIPLES;
/// assert_eq!(GRAPH_STAT_TRIPLES, "triples");
/// ```
pub const GRAPH_STAT_TRIPLES: &str = "triples";

/// JSON key for the class count in `stats()` results.
///
/// # Examples
///
/// ```
/// use open_ontologies::graph::GRAPH_STAT_CLASSES;
/// assert_eq!(GRAPH_STAT_CLASSES, "classes");
/// ```
pub const GRAPH_STAT_CLASSES: &str = "classes";

/// In-memory RDF graph store backed by Oxigraph.
///
/// # Examples
///
/// ```
/// use open_ontologies::graph::GraphStore;
///
/// let store = GraphStore::new();
/// assert_eq!(store.triple_count(), 0);
/// ```
pub struct GraphStore {
    store: Mutex<Store>,
}

impl Default for GraphStore {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphStore {
    /// Create a new empty in-memory graph store.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// let store = GraphStore::new();
    /// assert_eq!(store.triple_count(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            store: Mutex::new(Store::new().expect("Failed to create Oxigraph store")),
        }
    }

    /// Open a file-backed Oxigraph store at `path`. Used by the CLI so that
    /// successive `open-ontologies` subprocess invocations sharing the same
    /// `--data_dir` operate on the same persistent triple set. Falls back to
    /// an in-memory store if the on-disk store cannot be opened (e.g. the
    /// directory exists but is locked by another process).
    ///
    /// # Note
    ///
    /// This constructor requires a real filesystem path. Use [`GraphStore::new`]
    /// for hermetic in-memory usage.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use open_ontologies::graph::GraphStore;
    ///
    /// let store = GraphStore::open("/tmp/my_store").unwrap();
    /// ```
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let store = Store::open(path.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to open Oxigraph store at {:?}: {e}", path.as_ref()))?;
        Ok(Self {
            store: Mutex::new(store),
        })
    }

    /// Return the number of triples currently held in the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// let store = GraphStore::new();
    /// assert_eq!(store.triple_count(), 0);
    ///
    /// store.load_turtle(
    ///     "@prefix ex: <http://example.org/> . ex:A a <http://www.w3.org/2002/07/owl#Class> .",
    ///     None,
    /// ).unwrap();
    /// assert_eq!(store.triple_count(), 1);
    /// ```
    pub fn triple_count(&self) -> usize {
        let store = self.store.lock().unwrap();
        store.len().unwrap_or(0)
    }

    /// Load Turtle-formatted RDF into the store and return the number of triples inserted.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// let count = store.load_turtle(
    ///     "@prefix ex: <http://example.org/> . ex:A a <http://www.w3.org/2002/07/owl#Class> .",
    ///     None,
    /// )?;
    /// assert_eq!(count, 1);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// An optional base IRI resolves relative IRIs in the document:
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// let count = store.load_turtle(
    ///     "<A> a <http://www.w3.org/2002/07/owl#Class> .",
    ///     Some("http://example.org/"),
    /// )?;
    /// assert_eq!(count, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_turtle(&self, ttl: &str, base_iri: Option<&str>) -> anyhow::Result<usize> {
        let store = self.store.lock().unwrap();
        let reader = Cursor::new(ttl.as_bytes());
        let mut parser = RdfParser::from_format(RdfFormat::Turtle);
        if let Some(base) = base_iri {
            parser = parser.with_base_iri(base)?;
        }
        let quads_iter = parser.for_reader(reader);
        let mut count = 0;
        for quad in quads_iter {
            store.insert(&quad?)?;
            count += 1;
        }
        Ok(count)
    }

    /// Load RDF content in a specified format (Turtle, RDF/XML, etc.)
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    /// use oxigraph::io::RdfFormat;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// let count = store.load_content(
    ///     "<http://example.org/A> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .",
    ///     RdfFormat::NTriples,
    /// )?;
    /// assert_eq!(count, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_content(&self, content: &str, format: RdfFormat) -> anyhow::Result<usize> {
        self.load_content_with_base(content, format, None)
    }

    /// Load RDF content with an optional base IRI for resolving relative IRIs.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    /// use oxigraph::io::RdfFormat;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// let count = store.load_content_with_base(
    ///     "<A> a <http://www.w3.org/2002/07/owl#Class> .",
    ///     RdfFormat::Turtle,
    ///     Some("http://example.org/"),
    /// )?;
    /// assert_eq!(count, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_content_with_base(&self, content: &str, format: RdfFormat, base_iri: Option<&str>) -> anyhow::Result<usize> {
        let store = self.store.lock().unwrap();
        let reader = Cursor::new(content.as_bytes());
        let mut parser = RdfParser::from_format(format);
        if let Some(base) = base_iri {
            parser = parser.with_base_iri(base)?;
        }
        let parser = parser.for_reader(reader);
        let mut count = 0;
        for quad in parser {
            store.insert(&quad?)?;
            count += 1;
        }
        Ok(count)
    }

    /// Load an RDF file from disk, detecting the format from the file extension.
    ///
    /// Supported extensions: `.ttl` / `.turtle` (Turtle), `.nt` / `.ntriples`
    /// (N-Triples), `.rdf` / `.xml` / `.owl` (RDF/XML), `.nq` (N-Quads),
    /// `.trig` (TriG). Unknown extensions fall back to Turtle.
    ///
    /// # Note
    ///
    /// Requires a real filesystem path. For hermetic usage prefer
    /// [`GraphStore::load_turtle`] or [`GraphStore::load_content`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// let count = store.load_file("/path/to/ontology.ttl")?;
    /// println!("Loaded {count} triples");
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_file(&self, path: &str) -> anyhow::Result<usize> {
        let content = std::fs::read_to_string(path)?;
        let format = Self::detect_format(path);
        let store = self.store.lock().unwrap();
        let reader = Cursor::new(content.as_bytes());
        let parser = RdfParser::from_format(format).for_reader(reader);
        let mut count = 0;
        for quad in parser {
            store.insert(&quad?)?;
            count += 1;
        }
        Ok(count)
    }

    /// Serialize the store to a file in the given format.
    ///
    /// Supported format strings: `"turtle"` / `"ttl"`, `"ntriples"` / `"nt"`,
    /// `"rdfxml"` / `"rdf"` / `"xml"` / `"owl"`, `"nquads"` / `"nq"`, `"trig"`.
    ///
    /// # Note
    ///
    /// Requires a writable filesystem path. For hermetic serialization use
    /// [`GraphStore::serialize`] instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// store.load_turtle(
    ///     "@prefix ex: <http://example.org/> . ex:A a <http://www.w3.org/2002/07/owl#Class> .",
    ///     None,
    /// )?;
    /// store.save_file("/tmp/output.ttl", "turtle")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn save_file(&self, path: &str, format: &str) -> anyhow::Result<()> {
        let content = self.serialize(format)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Parse and validate Turtle-formatted RDF without loading it into any store.
    ///
    /// Returns the number of triples found if parsing succeeds, or an error
    /// describing the first parse failure. This is a **static method** — no
    /// `GraphStore` instance is required.
    ///
    /// # Examples
    ///
    /// Valid minimal Turtle with a declared prefix:
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let count = GraphStore::validate_turtle(
    ///     "@prefix ex: <http://example.org/> . ex:A a <http://www.w3.org/2002/07/owl#Class> .",
    /// )?;
    /// assert_eq!(count, 1);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Multiple triples all count:
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let ttl = "
    ///     @prefix ex: <http://example.org/> .
    ///     @prefix owl: <http://www.w3.org/2002/07/owl#> .
    ///     ex:A a owl:Class .
    ///     ex:B a owl:Class .
    ///     ex:B <http://www.w3.org/2000/01/rdf-schema#subClassOf> ex:A .
    /// ";
    /// let count = GraphStore::validate_turtle(ttl)?;
    /// assert_eq!(count, 3);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Invalid Turtle (missing terminating dot) returns an error:
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// let result = GraphStore::validate_turtle(
    ///     "@prefix ex: <http://example.org/> . ex:A a <http://www.w3.org/2002/07/owl#Class>",
    /// );
    /// assert!(result.is_err(), "truncated Turtle must not parse successfully");
    /// ```
    ///
    /// Completely malformed input is also rejected:
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// let result = GraphStore::validate_turtle("{ this is not rdf }");
    /// assert!(result.is_err(), "garbage input must not parse successfully");
    /// ```
    pub fn validate_turtle(ttl: &str) -> anyhow::Result<usize> {
        let reader = Cursor::new(ttl.as_bytes());
        let parser = RdfParser::from_format(RdfFormat::Turtle).for_reader(reader);
        let mut count = 0;
        for quad in parser {
            quad?;
            count += 1;
        }
        Ok(count)
    }

    /// Parse and validate an RDF file on disk without loading it into any store.
    ///
    /// Detects format from the file extension (same rules as [`GraphStore::load_file`]).
    /// Returns the triple count on success or a parse/IO error.
    ///
    /// # Note
    ///
    /// Requires a real filesystem path. For hermetic validation use
    /// [`GraphStore::validate_turtle`] or [`GraphStore::load_content`] instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let count = GraphStore::validate_file("/path/to/ontology.ttl")?;
    /// println!("File contains {count} valid triples");
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate_file(path: &str) -> anyhow::Result<usize> {
        let content = std::fs::read_to_string(path)?;
        let format = Self::detect_format(path);
        let reader = Cursor::new(content.as_bytes());
        let parser = RdfParser::from_format(format).for_reader(reader);
        let mut count = 0;
        for quad in parser {
            quad?;
            count += 1;
        }
        Ok(count)
    }

    /// Execute a SPARQL query (SELECT, CONSTRUCT, or ASK) against the store.
    ///
    /// Returns a JSON string. For SELECT queries the JSON object has
    /// `"variables"` (array of variable names) and `"results"` (array of
    /// binding maps). For ASK queries it has `"result"` (bool). For CONSTRUCT
    /// queries it has `"triples"` (array of subject/predicate/object maps).
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// store.load_turtle(
    ///     "@prefix ex: <http://example.org/> . ex:A a <http://www.w3.org/2002/07/owl#Class> .",
    ///     None,
    /// )?;
    ///
    /// let json = store.sparql_select(
    ///     "SELECT ?s WHERE { ?s a <http://www.w3.org/2002/07/owl#Class> }"
    /// )?;
    /// assert!(json.contains("http://example.org/A"));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ASK query returns a boolean result:
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// let json = store.sparql_select("ASK { ?s ?p ?o }")?;
    /// // Empty store — no triples, so ASK must return false
    /// assert!(json.contains("false"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn sparql_select(&self, query: &str) -> anyhow::Result<String> {
        let store = self.store.lock().unwrap();
        match store.query(query)? {
            QueryResults::Solutions(solutions) => {
                let vars: Vec<String> = solutions
                    .variables()
                    .iter()
                    .map(|v| v.as_str().to_string())
                    .collect();
                let mut rows: Vec<serde_json::Value> = Vec::new();
                for solution in solutions {
                    let solution = solution?;
                    let mut row = serde_json::Map::new();
                    for var in &vars {
                        if let Some(term) = solution.get(var.as_str()) {
                            row.insert(var.clone(), serde_json::Value::String(term.to_string()));
                        }
                    }
                    rows.push(serde_json::Value::Object(row));
                }
                Ok(serde_json::json!({"variables": vars, "results": rows}).to_string())
            }
            QueryResults::Boolean(b) => Ok(serde_json::json!({"result": b}).to_string()),
            QueryResults::Graph(triples) => {
                let mut result = Vec::new();
                for triple in triples {
                    let triple = triple?;
                    result.push(serde_json::json!({
                        "subject": triple.subject.to_string(),
                        "predicate": triple.predicate.to_string(),
                        "object": triple.object.to_string(),
                    }));
                }
                Ok(serde_json::json!({"triples": result}).to_string())
            }
        }
    }

    /// Run a SPARQL UPDATE (INSERT DATA / DELETE DATA / etc.) against the store.
    ///
    /// Returns the net number of new triples added (after minus before). A
    /// pure DELETE that removes triples will return `0` via saturating
    /// subtraction.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// let delta = store.sparql_update(
    ///     "INSERT DATA { <http://example.org/A> a <http://www.w3.org/2002/07/owl#Class> . }"
    /// )?;
    /// assert_eq!(delta, 1);
    /// assert_eq!(store.triple_count(), 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn sparql_update(&self, update: &str) -> anyhow::Result<usize> {
        let store = self.store.lock().unwrap();
        let before = store.len()?;
        store.update(update)?;
        let after = store.len()?;
        Ok(after.saturating_sub(before))
    }

    /// Serialize all triples in the store to a string in the given format.
    ///
    /// Supported format strings: `"turtle"` / `"ttl"`, `"ntriples"` / `"nt"`,
    /// `"rdfxml"` / `"rdf"` / `"xml"` / `"owl"`, `"nquads"` / `"nq"`, `"trig"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// store.load_turtle(
    ///     "@prefix ex: <http://example.org/> . ex:A a <http://www.w3.org/2002/07/owl#Class> .",
    ///     None,
    /// )?;
    /// let nt = store.serialize("ntriples")?;
    /// assert!(nt.contains("<http://example.org/A>"));
    /// assert!(nt.contains("<http://www.w3.org/2002/07/owl#Class>"));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// An unknown format name returns an error:
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// let store = GraphStore::new();
    /// assert!(store.serialize("jsonld").is_err());
    /// ```
    pub fn serialize(&self, format: &str) -> anyhow::Result<String> {
        let store = self.store.lock().unwrap();
        let rdf_format = Self::parse_format(format)?;
        let mut buf = Vec::new();
        let mut serializer = RdfSerializer::from_format(rdf_format).for_writer(&mut buf);
        for quad in store.iter() {
            let quad = quad?;
            serializer.serialize_triple(quad.as_ref())?;
        }
        drop(serializer);
        Ok(String::from_utf8(buf)?)
    }

    /// Return a JSON summary of the store: total triples, class count,
    /// property count, and individual count.
    ///
    /// # Examples
    ///
    /// An empty store reports zero for every field:
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// let json = store.get_stats()?;
    /// let v: serde_json::Value = serde_json::from_str(&json)?;
    /// assert_eq!(v["triples"], 0);
    /// assert_eq!(v["classes"], 0);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// After loading a class triple the counts are non-zero:
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// store.load_turtle(
    ///     "@prefix owl: <http://www.w3.org/2002/07/owl#> .\
    ///      \n<urn:ex:A> a owl:Class .",
    ///     None,
    /// )?;
    /// let json = store.get_stats()?;
    /// let v: serde_json::Value = serde_json::from_str(&json)?;
    /// assert!(v["triples"].as_u64().unwrap() >= 1);
    /// assert!(v["classes"].as_u64().unwrap() >= 1);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// The returned JSON always contains the `"triples"` key:
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// let json = store.get_stats()?;
    /// let v: serde_json::Value = serde_json::from_str(&json)?;
    /// assert!(v.get("triples").is_some());
    /// assert!(v.get("classes").is_some());
    /// assert!(v.get("properties").is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_stats(&self) -> anyhow::Result<String> {
        let store = self.store.lock().unwrap();
        let total = store.len()?;

        // Count classes: explicit type declarations + implicit (subClassOf subjects/objects,
        // domain/range targets, equivalentClass). Filters out blank nodes and OWL/RDF builtins.
        let class_query = "SELECT (COUNT(DISTINCT ?c) AS ?count) WHERE {
            { ?c a <http://www.w3.org/2002/07/owl#Class> }
            UNION { ?c a <http://www.w3.org/2000/01/rdf-schema#Class> }
            UNION { ?c <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?p }
            UNION { ?p <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?c }
            UNION { ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?c }
            UNION { ?p <http://www.w3.org/2000/01/rdf-schema#range> ?c }
            UNION { ?c <http://www.w3.org/2002/07/owl#equivalentClass> ?p }
            FILTER(isIRI(?c)
                && ?c != <http://www.w3.org/2002/07/owl#Thing>
                && ?c != <http://www.w3.org/2002/07/owl#Nothing>
                && ?c != <http://www.w3.org/2000/01/rdf-schema#Resource>
                && ?c != <http://www.w3.org/2000/01/rdf-schema#Literal>
                && ?c != <http://www.w3.org/2000/01/rdf-schema#Class>
                && ?c != <http://www.w3.org/2002/07/owl#Class>)
        }";
        // Count properties: explicit type + implicit (subPropertyOf, domain/range subjects)
        let prop_query = "SELECT (COUNT(DISTINCT ?p) AS ?count) WHERE {
            { ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> }
            UNION { ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> }
            UNION { ?p a <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property> }
            UNION { ?p <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?q }
            UNION { ?q <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> ?p }
            UNION { ?p <http://www.w3.org/2000/01/rdf-schema#domain> ?c }
            UNION { ?p <http://www.w3.org/2000/01/rdf-schema#range> ?c }
            FILTER(isIRI(?p)
                && !STRSTARTS(STR(?p), \"http://www.w3.org/1999/02/22-rdf-syntax-ns#\")
                && !STRSTARTS(STR(?p), \"http://www.w3.org/2000/01/rdf-schema#\")
                && !STRSTARTS(STR(?p), \"http://www.w3.org/2002/07/owl#\"))
        }";
        let individual_query = "SELECT (COUNT(DISTINCT ?i) AS ?count) WHERE { ?i a ?c . FILTER(?c != <http://www.w3.org/2002/07/owl#Class> && ?c != <http://www.w3.org/2000/01/rdf-schema#Class> && ?c != <http://www.w3.org/2002/07/owl#ObjectProperty> && ?c != <http://www.w3.org/2002/07/owl#DatatypeProperty> && ?c != <http://www.w3.org/2002/07/owl#Ontology>) }";

        let count_from_query = |q: &str| -> usize {
            let Ok(QueryResults::Solutions(solutions)) = store.query(q) else { return 0 };
            let Some(Ok(row)) = solutions.into_iter().next() else { return 0 };
            let Some(Term::Literal(lit)) = row.get("count") else { return 0 };
            lit.value().parse().unwrap_or(0)
        };

        let classes = count_from_query(class_query);
        let props = count_from_query(prop_query);
        let individuals = count_from_query(individual_query);

        Ok(serde_json::json!({
            "triples": total,
            "classes": classes,
            "object_properties": props,
            "data_properties": 0,
            "properties": props,
            "individuals": individuals
        })
        .to_string())
    }

    /// Remove all triples from the store, returning it to an empty state.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// store.load_turtle(
    ///     "@prefix ex: <http://example.org/> . ex:A a <http://www.w3.org/2002/07/owl#Class> .",
    ///     None,
    /// )?;
    /// assert_eq!(store.triple_count(), 1);
    ///
    /// store.clear()?;
    /// assert_eq!(store.triple_count(), 0);
    /// # Ok(())
    /// # }
    /// ```
    pub fn clear(&self) -> anyhow::Result<()> {
        let store = self.store.lock().unwrap();
        store.clear()?;
        Ok(())
    }

    /// Load N-Triples–formatted RDF into the store and return the number of
    /// triples inserted.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// let count = store.load_ntriples(
    ///     "<http://example.org/A> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .\n"
    /// )?;
    /// assert_eq!(count, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_ntriples(&self, content: &str) -> anyhow::Result<usize> {
        let store = self.store.lock().unwrap();
        let reader = Cursor::new(content.as_bytes());
        let parser = RdfParser::from_format(RdfFormat::NTriples).for_reader(reader);
        let mut count = 0;
        for quad in parser {
            store.insert(&quad?)?;
            count += 1;
        }
        Ok(count)
    }

    /// Alias for [`GraphStore::serialize`] — produces a full serialization of
    /// the store in the given format string.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// store.load_ntriples(
    ///     "<http://example.org/A> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .\n"
    /// )?;
    /// let snap = store.snapshot("ntriples")?;
    /// assert!(snap.contains("<http://example.org/A>"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn snapshot(&self, format: &str) -> anyhow::Result<String> {
        self.serialize(format)
    }

    /// Fetch raw text from an HTTP URL. Typically used to download remote
    /// ontologies before parsing.
    ///
    /// # Note
    ///
    /// Requires a live network connection. Use [`GraphStore::load_turtle`] for
    /// hermetic in-memory usage.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use open_ontologies::graph::GraphStore;
    ///
    /// async fn example() {
    ///     let ttl = GraphStore::fetch_url("https://example.org/ontology.ttl").await.unwrap();
    ///     let store = GraphStore::new();
    ///     store.load_turtle(&ttl, None).unwrap();
    /// }
    /// ```
    pub async fn fetch_url(url: &str) -> anyhow::Result<String> {
        let resp = reqwest::get(url).await?;
        if !resp.status().is_success() {
            anyhow::bail!("HTTP {}: {}", resp.status(), url);
        }
        Ok(resp.text().await?)
    }

    /// Send a SPARQL CONSTRUCT query to a remote SPARQL endpoint and return the
    /// response body (typically Turtle or N-Triples).
    ///
    /// # Note
    ///
    /// Requires a live network connection to a SPARQL endpoint.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use open_ontologies::graph::GraphStore;
    ///
    /// async fn example() {
    ///     let ttl = GraphStore::fetch_sparql(
    ///         "https://dbpedia.org/sparql",
    ///         "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o } LIMIT 10",
    ///     ).await.unwrap();
    ///     println!("{ttl}");
    /// }
    /// ```
    pub async fn fetch_sparql(endpoint: &str, query: &str) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        let resp = client
            .post(endpoint)
            .header("Content-Type", "application/sparql-query")
            .header("Accept", "text/turtle")
            .body(query.to_string())
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("SPARQL endpoint returned HTTP {}", resp.status());
        }
        Ok(resp.text().await?)
    }

    /// Push N-Triples content to a remote SPARQL 1.1 Update endpoint using
    /// `INSERT DATA { … }` into the default graph. Delegates to
    /// [`GraphStore::push_sparql_graph`] with `graph_iri = None` and no extra
    /// headers.
    ///
    /// # Note
    ///
    /// Requires a live network connection to a SPARQL Update endpoint.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use open_ontologies::graph::GraphStore;
    ///
    /// async fn example() {
    ///     GraphStore::push_sparql(
    ///         "http://localhost:7200/repositories/myrepo/statements",
    ///         "<http://example.org/A> a <http://www.w3.org/2002/07/owl#Class> .",
    ///     ).await.unwrap();
    /// }
    /// ```
    pub async fn push_sparql(endpoint: &str, content: &str) -> anyhow::Result<String> {
        Self::push_sparql_graph(endpoint, content, None, &[]).await
    }

    /// SPARQL 1.1 Update push with optional named graph and arbitrary extra
    /// HTTP headers. When `graph_iri` is `Some(iri)`, the body becomes
    /// `INSERT DATA { GRAPH <iri> { ntriples } }`; when `None`, the default
    /// graph form is used.
    ///
    /// `extra_headers` is a slice of `(name, value)` pairs prepended to the
    /// request — used by OntoStar to bind a receipt hash to the push via
    /// `X-Ostar-Receipt-Hash` and `X-Ostar-Production-Law`.
    ///
    /// Validates `graph_iri` syntactically: must be non-empty, must not be
    /// wrapped in `< >` (caller passes the raw IRI), and must not contain
    /// whitespace.
    pub async fn push_sparql_graph(
        endpoint: &str,
        content: &str,
        graph_iri: Option<&str>,
        extra_headers: &[(&str, &str)],
    ) -> anyhow::Result<String> {
        let body = match graph_iri {
            None => format!("INSERT DATA {{ {} }}", content),
            Some(iri) => {
                let trimmed = iri.trim();
                if trimmed.is_empty() {
                    anyhow::bail!("graph IRI must not be empty");
                }
                if trimmed.starts_with('<') || trimmed.ends_with('>') {
                    anyhow::bail!(
                        "graph IRI must not be wrapped in angle brackets (got '{}')",
                        iri
                    );
                }
                if trimmed.chars().any(|c| c.is_whitespace()) {
                    anyhow::bail!("graph IRI must not contain whitespace (got '{}')", iri);
                }
                format!("INSERT DATA {{ GRAPH <{}> {{ {} }} }}", trimmed, content)
            }
        };
        let client = reqwest::Client::new();
        let mut req = client
            .post(endpoint)
            .header("Content-Type", "application/sparql-update");
        for (name, value) in extra_headers {
            req = req.header(*name, *value);
        }
        let resp = req.body(body).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("SPARQL update returned HTTP {}", resp.status());
        }
        Ok(format!("Pushed to {}: HTTP {}", endpoint, resp.status()))
    }

    /// Extract all triples as `(subject, predicate, object)` string tuples.
    ///
    /// Each term is rendered in canonical Oxigraph notation: IRIs as
    /// `<http://...>`, literals as `"value"^^<datatype>`, and blank nodes as
    /// `_:id`.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::graph::GraphStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let store = GraphStore::new();
    /// store.load_turtle(
    ///     "@prefix ex: <http://example.org/> . ex:A a <http://www.w3.org/2002/07/owl#Class> .",
    ///     None,
    /// )?;
    ///
    /// let triples = store.all_triples()?;
    /// assert_eq!(triples.len(), 1);
    ///
    /// let (s, p, o) = &triples[0];
    /// assert_eq!(s, "<http://example.org/A>");
    /// assert_eq!(p, "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>");
    /// assert_eq!(o, "<http://www.w3.org/2002/07/owl#Class>");
    /// # Ok(())
    /// # }
    /// ```
    pub fn all_triples(&self) -> anyhow::Result<Vec<(String, String, String)>> {
        let store = self.store.lock().unwrap();
        let mut triples = Vec::new();
        for quad in store.iter() {
            let quad = quad?;
            let s = quad.subject.to_string();
            let p = quad.predicate.to_string();
            let o = quad.object.to_string();
            triples.push((s, p, o));
        }
        Ok(triples)
    }

    fn detect_format(path: &str) -> RdfFormat {
        if path.ends_with(".ttl") || path.ends_with(".turtle") {
            RdfFormat::Turtle
        } else if path.ends_with(".nt") || path.ends_with(".ntriples") {
            RdfFormat::NTriples
        } else if path.ends_with(".rdf") || path.ends_with(".xml") || path.ends_with(".owl") {
            RdfFormat::RdfXml
        } else if path.ends_with(".nq") {
            RdfFormat::NQuads
        } else if path.ends_with(".trig") {
            RdfFormat::TriG
        } else {
            RdfFormat::Turtle
        }
    }

    fn parse_format(name: &str) -> anyhow::Result<RdfFormat> {
        match name.to_lowercase().as_str() {
            GRAPH_FORMAT_TURTLE | "ttl" => Ok(RdfFormat::Turtle),
            GRAPH_FORMAT_NTRIPLES | "nt" => Ok(RdfFormat::NTriples),
            GRAPH_FORMAT_RDFXML | "rdf" | "xml" | "owl" => Ok(RdfFormat::RdfXml),
            GRAPH_FORMAT_NQUADS | "nq" => Ok(RdfFormat::NQuads),
            GRAPH_FORMAT_TRIG => Ok(RdfFormat::TriG),
            _ => anyhow::bail!(
                "Unknown format: {}. Supported: turtle, ntriples, rdfxml, nquads, trig",
                name
            ),
        }
    }
}
