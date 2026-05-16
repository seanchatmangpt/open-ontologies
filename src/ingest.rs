use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;

/// Canonical format token for CSV — the default when no extension is recognised.
pub const FORMAT_CSV: &str = "csv";
/// Canonical format token for JSON.
pub const FORMAT_JSON: &str = "json";
/// Canonical format token for newline-delimited JSON (NDJSON / JSON Lines).
pub const FORMAT_NDJSON: &str = "ndjson";
/// Canonical format token for XML.
pub const FORMAT_XML: &str = "xml";
/// Canonical format token for YAML.
pub const FORMAT_YAML: &str = "yaml";
/// Canonical format token for Excel XLSX.
pub const FORMAT_XLSX: &str = "xlsx";
/// Canonical format token for Apache Parquet.
pub const FORMAT_PARQUET: &str = "parquet";

/// Data ingester that parses structured data files into rows of key-value pairs.
pub struct DataIngester;

/// Convert a serde_json::Value to a flat string representation.
fn json_value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

impl DataIngester {
    /// Detect the file format from the file extension.
    /// Returns one of: "csv", "json", "ndjson", "xml", "yaml", "xlsx", "parquet".
    /// Defaults to "csv" if the extension is unrecognised.
    ///
    /// # Examples
    ///
    /// ```
    /// # use open_ontologies::ingest::DataIngester;
    /// assert_eq!(DataIngester::detect_format("data.csv"), "csv");
    /// assert_eq!(DataIngester::detect_format("data.json"), "json");
    /// assert_eq!(DataIngester::detect_format("events.ndjson"), "ndjson");
    /// assert_eq!(DataIngester::detect_format("events.jsonl"), "ndjson");
    /// assert_eq!(DataIngester::detect_format("schema.xml"), "xml");
    /// assert_eq!(DataIngester::detect_format("config.yaml"), "yaml");
    /// assert_eq!(DataIngester::detect_format("config.yml"), "yaml");
    /// assert_eq!(DataIngester::detect_format("report.xlsx"), "xlsx");
    /// assert_eq!(DataIngester::detect_format("data.parquet"), "parquet");
    /// // Unknown extensions default to csv
    /// assert_eq!(DataIngester::detect_format("unknown.xyz"), "csv");
    /// assert_eq!(DataIngester::detect_format("no_extension"), "csv");
    /// ```
    pub fn detect_format(path: &str) -> &'static str {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "csv" => FORMAT_CSV,
            "json" => FORMAT_JSON,
            "jsonl" | "ndjson" => FORMAT_NDJSON,
            "xml" => FORMAT_XML,
            "yaml" | "yml" => FORMAT_YAML,
            "xlsx" => FORMAT_XLSX,
            "parquet" => FORMAT_PARQUET,
            _ => FORMAT_CSV,
        }
    }

    /// Parse CSV content with headers into rows.
    ///
    /// The first row is treated as the header. Each subsequent row becomes a
    /// `HashMap<String, String>` keyed by header name.
    ///
    /// **Process-mining note:** These field names become OCEL attribute keys
    /// downstream. A typo here causes a silent event-attribute drop — the most
    /// common source of missing attributes in mined logs.
    ///
    /// # Examples
    ///
    /// ```
    /// # use open_ontologies::ingest::DataIngester;
    /// let rows = DataIngester::parse_csv("name,age\nAlice,30\nBob,25").unwrap();
    /// assert_eq!(rows.len(), 2);
    /// assert_eq!(rows[0]["name"], "Alice");
    /// assert_eq!(rows[0]["age"], "30");
    /// assert_eq!(rows[1]["name"], "Bob");
    /// assert_eq!(rows[1]["age"], "25");
    /// ```
    ///
    /// Empty content (headers only) produces no rows:
    ///
    /// ```
    /// # use open_ontologies::ingest::DataIngester;
    /// let rows = DataIngester::parse_csv("name,age\n").unwrap();
    /// assert_eq!(rows.len(), 0);
    /// ```
    pub fn parse_csv(content: &str) -> Result<Vec<HashMap<String, String>>> {
        let mut reader = csv::Reader::from_reader(content.as_bytes());
        let headers: Vec<String> = reader
            .headers()
            .context("Failed to read CSV headers")?
            .iter()
            .map(|h| h.to_string())
            .collect();

        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result.context("Failed to read CSV record")?;
            let mut row = HashMap::new();
            for (i, value) in record.iter().enumerate() {
                if let Some(key) = headers.get(i) {
                    row.insert(key.clone(), value.to_string());
                }
            }
            rows.push(row);
        }
        Ok(rows)
    }

    /// Parse a JSON string. Accepts a JSON array of objects or a single object.
    ///
    /// Scalar values (string, number, bool) are stringified. `null` becomes an
    /// empty string. Nested objects and arrays are serialised via their `Display`
    /// representation.
    ///
    /// # Examples
    ///
    /// Array of objects:
    ///
    /// ```
    /// # use open_ontologies::ingest::DataIngester;
    /// let json = r#"[{"city":"Chicago","pop":2700000},{"city":"Austin","pop":978908}]"#;
    /// let rows = DataIngester::parse_json(json).unwrap();
    /// assert_eq!(rows.len(), 2);
    /// assert_eq!(rows[0]["city"], "Chicago");
    /// assert_eq!(rows[0]["pop"], "2700000");
    /// ```
    ///
    /// Single object is wrapped in a one-element vec:
    ///
    /// ```
    /// # use open_ontologies::ingest::DataIngester;
    /// let json = r#"{"name":"Alice","active":true}"#;
    /// let rows = DataIngester::parse_json(json).unwrap();
    /// assert_eq!(rows.len(), 1);
    /// assert_eq!(rows[0]["name"], "Alice");
    /// assert_eq!(rows[0]["active"], "true");
    /// ```
    pub fn parse_json(content: &str) -> Result<Vec<HashMap<String, String>>> {
        let value: serde_json::Value =
            serde_json::from_str(content).context("Failed to parse JSON")?;
        match value {
            serde_json::Value::Array(arr) => {
                let mut rows = Vec::new();
                for item in &arr {
                    if let serde_json::Value::Object(map) = item {
                        let row: HashMap<String, String> = map
                            .iter()
                            .map(|(k, v)| (k.clone(), json_value_to_string(v)))
                            .collect();
                        rows.push(row);
                    }
                }
                Ok(rows)
            }
            serde_json::Value::Object(map) => {
                let row: HashMap<String, String> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), json_value_to_string(v)))
                    .collect();
                Ok(vec![row])
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Parse newline-delimited JSON (NDJSON / JSON Lines).
    ///
    /// Each non-empty line must be a JSON object. Blank lines are skipped.
    ///
    /// # Examples
    ///
    /// ```
    /// # use open_ontologies::ingest::DataIngester;
    /// let ndjson = "{\"id\":\"e1\",\"action\":\"start\"}\n{\"id\":\"e2\",\"action\":\"stop\"}";
    /// let rows = DataIngester::parse_ndjson(ndjson).unwrap();
    /// assert_eq!(rows.len(), 2);
    /// assert_eq!(rows[0]["id"], "e1");
    /// assert_eq!(rows[1]["action"], "stop");
    /// ```
    ///
    /// Blank lines are ignored:
    ///
    /// ```
    /// # use open_ontologies::ingest::DataIngester;
    /// let ndjson = "{\"x\":\"1\"}\n\n{\"x\":\"2\"}\n";
    /// let rows = DataIngester::parse_ndjson(ndjson).unwrap();
    /// assert_eq!(rows.len(), 2);
    /// ```
    pub fn parse_ndjson(content: &str) -> Result<Vec<HashMap<String, String>>> {
        let mut rows = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_str(trimmed).context("Failed to parse NDJSON line")?;
            if let serde_json::Value::Object(map) = value {
                let row: HashMap<String, String> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), json_value_to_string(v)))
                    .collect();
                rows.push(row);
            }
        }
        Ok(rows)
    }

    /// Parse a YAML string containing an array of objects.
    ///
    /// The top-level value must be a YAML sequence. Each element must be a
    /// mapping. Scalar values are stringified; `null` becomes an empty string.
    ///
    /// # Examples
    ///
    /// ```
    /// # use open_ontologies::ingest::DataIngester;
    /// let yaml = "- name: Alice\n  score: 95\n- name: Bob\n  score: 87\n";
    /// let rows = DataIngester::parse_yaml(yaml).unwrap();
    /// assert_eq!(rows.len(), 2);
    /// assert_eq!(rows[0]["name"], "Alice");
    /// assert_eq!(rows[0]["score"], "95");
    /// assert_eq!(rows[1]["name"], "Bob");
    /// ```
    ///
    /// A non-sequence top-level (e.g. a bare mapping) produces an empty vec:
    ///
    /// ```
    /// # use open_ontologies::ingest::DataIngester;
    /// let yaml = "name: Alice\nscore: 95\n";
    /// let rows = DataIngester::parse_yaml(yaml).unwrap();
    /// assert_eq!(rows.len(), 0);
    /// ```
    pub fn parse_yaml(content: &str) -> Result<Vec<HashMap<String, String>>> {
        let value: serde_yaml::Value =
            serde_yaml::from_str(content).context("Failed to parse YAML")?;
        match value {
            serde_yaml::Value::Sequence(seq) => {
                let mut rows = Vec::new();
                for item in &seq {
                    if let serde_yaml::Value::Mapping(map) = item {
                        let mut row = HashMap::new();
                        for (k, v) in map {
                            let key = match k {
                                serde_yaml::Value::String(s) => s.clone(),
                                other => format!("{other:?}"),
                            };
                            let val = match v {
                                serde_yaml::Value::String(s) => s.clone(),
                                serde_yaml::Value::Number(n) => n.to_string(),
                                serde_yaml::Value::Bool(b) => b.to_string(),
                                serde_yaml::Value::Null => String::new(),
                                other => format!("{other:?}"),
                            };
                            row.insert(key, val);
                        }
                        rows.push(row);
                    }
                }
                Ok(rows)
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Parse XML with a `<root><record>...</record></root>` structure.
    /// Depth 1 = root element, depth 2 = record boundary, depth 3 = field elements.
    ///
    /// The root element name is arbitrary. Each depth-2 child is a record; each
    /// depth-3 child is a field. Text content of the field becomes the value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use open_ontologies::ingest::DataIngester;
    /// let xml = "<root><item><name>Alice</name><age>30</age></item>\
    ///            <item><name>Bob</name><age>25</age></item></root>";
    /// let rows = DataIngester::parse_xml(xml).unwrap();
    /// assert_eq!(rows.len(), 2);
    /// assert_eq!(rows[0]["name"], "Alice");
    /// assert_eq!(rows[0]["age"], "30");
    /// assert_eq!(rows[1]["name"], "Bob");
    /// ```
    ///
    /// Empty root produces no rows:
    ///
    /// ```
    /// # use open_ontologies::ingest::DataIngester;
    /// let xml = "<root></root>";
    /// let rows = DataIngester::parse_xml(xml).unwrap();
    /// assert_eq!(rows.len(), 0);
    /// ```
    pub fn parse_xml(content: &str) -> Result<Vec<HashMap<String, String>>> {
        use quick_xml::events::Event;
        use quick_xml::reader::Reader;

        let mut reader = Reader::from_str(content);
        reader.config_mut().trim_text(true);

        let mut rows: Vec<HashMap<String, String>> = Vec::new();
        let mut current_row: Option<HashMap<String, String>> = None;
        let mut current_field: Option<String> = None;
        let mut depth: u32 = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    depth += 1;
                    match depth {
                        1 => {
                            // Root element — nothing to do
                        }
                        2 => {
                            // Record start
                            current_row = Some(HashMap::new());
                        }
                        3 => {
                            // Field element start
                            let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                            current_field = Some(name);
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    if depth == 3
                        && let (Some(row), Some(field)) =
                            (&mut current_row, &current_field)
                        {
                            let text = e.unescape().unwrap_or_default().to_string();
                            row.insert(field.clone(), text);
                        }
                }
                Ok(Event::End(_)) => {
                    match depth {
                        2 => {
                            // Record end — push row
                            if let Some(row) = current_row.take() {
                                rows.push(row);
                            }
                        }
                        3 => {
                            current_field = None;
                        }
                        _ => {}
                    }
                    depth -= 1;
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(anyhow::anyhow!("XML parse error: {e}")),
                _ => {}
            }
        }
        Ok(rows)
    }

    /// Parse an Excel (.xlsx) file. First row is treated as headers.
    ///
    /// Requires a real `.xlsx` file on disk; not suitable for a hermetic doctest.
    ///
    /// ```no_run
    /// # use open_ontologies::ingest::DataIngester;
    /// // Provide an actual xlsx path at runtime.
    /// let rows = DataIngester::parse_xlsx_file("/path/to/data.xlsx").unwrap();
    /// assert!(!rows.is_empty());
    /// ```
    pub fn parse_xlsx_file(path: &str) -> Result<Vec<HashMap<String, String>>> {
        use calamine::{open_workbook, Reader, Xlsx};

        let mut workbook: Xlsx<_> =
            open_workbook(path).context("Failed to open XLSX workbook")?;

        let sheet_names = workbook.sheet_names().to_vec();
        let first_sheet = sheet_names
            .first()
            .context("XLSX workbook has no sheets")?
            .clone();

        let range = workbook
            .worksheet_range(&first_sheet)
            .context("Failed to read XLSX worksheet")?;

        let mut row_iter = range.rows();
        let header_row = row_iter.next().context("XLSX file has no rows")?;

        let headers: Vec<String> = header_row
            .iter()
            .map(Self::calamine_cell_to_string)
            .collect();

        let mut rows = Vec::new();
        for data_row in row_iter {
            let mut row = HashMap::new();
            for (i, cell) in data_row.iter().enumerate() {
                if let Some(key) = headers.get(i) {
                    row.insert(key.clone(), Self::calamine_cell_to_string(cell));
                }
            }
            rows.push(row);
        }
        Ok(rows)
    }

    /// Convert a calamine Data cell to a string.
    fn calamine_cell_to_string(cell: &calamine::Data) -> String {
        match cell {
            calamine::Data::Int(i) => i.to_string(),
            calamine::Data::Float(f) => f.to_string(),
            calamine::Data::String(s) => s.clone(),
            calamine::Data::Bool(b) => b.to_string(),
            calamine::Data::DateTime(dt) => dt.to_string(),
            calamine::Data::DateTimeIso(s) => s.clone(),
            calamine::Data::DurationIso(s) => s.clone(),
            calamine::Data::Error(e) => format!("{e:?}"),
            calamine::Data::Empty => String::new(),
        }
    }

    /// Parse a Parquet file into rows.
    ///
    /// Requires a real `.parquet` file on disk; not suitable for a hermetic doctest.
    ///
    /// ```no_run
    /// # use open_ontologies::ingest::DataIngester;
    /// // Provide an actual parquet path at runtime.
    /// let rows = DataIngester::parse_parquet_file("/path/to/data.parquet").unwrap();
    /// assert!(!rows.is_empty());
    /// ```
    pub fn parse_parquet_file(path: &str) -> Result<Vec<HashMap<String, String>>> {
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        use std::fs::File;

        let file = File::open(path).context("Failed to open Parquet file")?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .context("Failed to create Parquet reader builder")?;

        let reader = builder.build().context("Failed to build Parquet reader")?;

        let mut rows = Vec::new();
        for batch_result in reader {
            let batch: arrow::array::RecordBatch =
                batch_result.context("Failed to read Parquet record batch")?;
            let schema = batch.schema();
            let num_rows = batch.num_rows();
            let num_cols = batch.num_columns();

            for row_idx in 0..num_rows {
                let mut row = HashMap::new();
                for col_idx in 0..num_cols {
                    let field = schema.field(col_idx);
                    let col = batch.column(col_idx);
                    let value = if col.is_null(row_idx) {
                        String::new()
                    } else {
                        Self::arrow_array_value_to_string(col.as_ref(), row_idx, field.data_type())
                    };
                    row.insert(field.name().clone(), value);
                }
                rows.push(row);
            }
        }
        Ok(rows)
    }

    /// Convert an Arrow array value at a given index to a string.
    fn arrow_array_value_to_string(
        array: &dyn arrow::array::Array,
        idx: usize,
        data_type: &arrow::datatypes::DataType,
    ) -> String {
        use arrow::array::*;
        use arrow::datatypes::DataType as ArrowType;

        match data_type {
            ArrowType::Boolean => {
                let a = array.as_any().downcast_ref::<BooleanArray>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Int8 => {
                let a = array.as_any().downcast_ref::<Int8Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Int16 => {
                let a = array.as_any().downcast_ref::<Int16Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Int32 => {
                let a = array.as_any().downcast_ref::<Int32Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Int64 => {
                let a = array.as_any().downcast_ref::<Int64Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::UInt8 => {
                let a = array.as_any().downcast_ref::<UInt8Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::UInt16 => {
                let a = array.as_any().downcast_ref::<UInt16Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::UInt32 => {
                let a = array.as_any().downcast_ref::<UInt32Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::UInt64 => {
                let a = array.as_any().downcast_ref::<UInt64Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Float32 => {
                let a = array.as_any().downcast_ref::<Float32Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Float64 => {
                let a = array.as_any().downcast_ref::<Float64Array>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::Utf8 => {
                let a = array.as_any().downcast_ref::<StringArray>().unwrap();
                a.value(idx).to_string()
            }
            ArrowType::LargeUtf8 => {
                let a = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
                a.value(idx).to_string()
            }
            _ => format!("{array:?}[{idx}]"),
        }
    }

    /// Dispatch to the correct parser based on detected format.
    /// For text formats, reads the file content first.
    ///
    /// Requires a real file on disk; not suitable for a hermetic doctest.
    ///
    /// ```no_run
    /// # use open_ontologies::ingest::DataIngester;
    /// // Extension-based dispatch: "data.csv" → parse_csv, "data.json" → parse_json, etc.
    /// let rows = DataIngester::parse_file("/path/to/data.csv").unwrap();
    /// assert!(!rows.is_empty());
    /// ```
    pub fn parse_file(path: &str) -> Result<Vec<HashMap<String, String>>> {
        Self::parse_file_with_format(path, None)
    }

    /// Parse with explicit format override. When `format` is `Some`, the
    /// supplied string takes precedence over extension-based detection.
    /// Accepted: csv, json, ndjson, yaml, xml, xlsx, parquet (case-insensitive).
    ///
    /// Reads from the filesystem; not suitable for a hermetic doctest. The
    /// signature is:
    ///
    /// ```no_run
    /// # use open_ontologies::ingest::DataIngester;
    /// // Override the format: treat a .dat file as CSV.
    /// let rows = DataIngester::parse_file_with_format(
    ///     "/path/to/data.dat",
    ///     Some("csv"),
    /// ).unwrap();
    /// assert!(!rows.is_empty());
    /// ```
    pub fn parse_file_with_format(
        path: &str,
        format: Option<&str>,
    ) -> Result<Vec<HashMap<String, String>>> {
        let detected = Self::detect_format(path);
        let format: &str = match format {
            None => detected,
            Some(f) => {
                let lower = f.to_ascii_lowercase();
                match lower.as_str() {
                    "csv" => FORMAT_CSV,
                    "json" => FORMAT_JSON,
                    "ndjson" => FORMAT_NDJSON,
                    "yaml" | "yml" => FORMAT_YAML,
                    "xml" => FORMAT_XML,
                    "xlsx" => FORMAT_XLSX,
                    "parquet" => FORMAT_PARQUET,
                    other => {
                        anyhow::bail!(
                            "unsupported format override '{}': expected one of csv, json, ndjson, yaml, xml, xlsx, parquet",
                            other
                        )
                    }
                }
            }
        };
        match format {
            "csv" => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {path}"))?;
                Self::parse_csv(&content)
            }
            "json" => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {path}"))?;
                Self::parse_json(&content)
            }
            "ndjson" => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {path}"))?;
                Self::parse_ndjson(&content)
            }
            "yaml" => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {path}"))?;
                Self::parse_yaml(&content)
            }
            "xml" => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {path}"))?;
                Self::parse_xml(&content)
            }
            "xlsx" => Self::parse_xlsx_file(path),
            "parquet" => Self::parse_parquet_file(path),
            _ => {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read file: {path}"))?;
                Self::parse_csv(&content)
            }
        }
    }

    /// Collect unique keys from all rows, sorted alphabetically.
    ///
    /// Scans every row for its keys and returns the union, deduplicated and
    /// sorted. Rows with sparse columns (missing keys compared to other rows)
    /// are handled gracefully — only present keys contribute.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::collections::HashMap;
    /// # use open_ontologies::ingest::DataIngester;
    /// let rows = DataIngester::parse_csv("name,age,city\nAlice,30,Chicago\nBob,25,Austin").unwrap();
    /// let headers = DataIngester::extract_headers(&rows);
    /// assert_eq!(headers, vec!["age", "city", "name"]);
    /// ```
    ///
    /// Sparse rows — keys from all rows are unioned:
    ///
    /// ```
    /// # use std::collections::HashMap;
    /// # use open_ontologies::ingest::DataIngester;
    /// let mut r1 = HashMap::new();
    /// r1.insert("a".to_string(), "1".to_string());
    /// let mut r2 = HashMap::new();
    /// r2.insert("b".to_string(), "2".to_string());
    /// let headers = DataIngester::extract_headers(&[r1, r2]);
    /// assert_eq!(headers, vec!["a", "b"]);
    /// ```
    ///
    /// Empty slice produces an empty vec:
    ///
    /// ```
    /// # use open_ontologies::ingest::DataIngester;
    /// let headers = DataIngester::extract_headers(&[]);
    /// assert!(headers.is_empty());
    /// ```
    pub fn extract_headers(rows: &[HashMap<String, String>]) -> Vec<String> {
        let mut keys: Vec<String> = rows
            .iter()
            .flat_map(|row| row.keys().cloned())
            .collect::<std::collections::HashSet<String>>()
            .into_iter()
            .collect();
        keys.sort();
        keys
    }
}
