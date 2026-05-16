//! Unified SQL data source abstraction.
//!
//! Provides a single entry point for the data pipeline to talk to multiple SQL
//! backbones. A connection string is dispatched to the matching driver based
//! on its scheme/prefix:
//!
//! | Prefix                                   | Driver       | Feature flag |
//! | ---------------------------------------- | ------------ | ------------ |
//! | `postgres://` / `postgresql://`          | PostgreSQL   | `postgres`   |
//! | `duckdb://` / `duckdb:` / `:memory:`     | DuckDB       | `duckdb`     |
//! | `*.duckdb` / `*.ddb` (file path)         | DuckDB       | `duckdb`     |
//!
//! The abstraction has two responsibilities:
//!
//!   1. **Schema introspection** — return [`crate::schema::TableInfo`] entries
//!      so the OWL generator can build classes, datatype/object properties,
//!      and cardinality restrictions from a relational schema. This powers
//!      `onto_import_schema` / `import-schema`.
//!
//!   2. **Row extraction via SQL query** — run a user-supplied SELECT and
//!      return rows as `Vec<HashMap<String,String>>`, exactly the shape the
//!      existing [`crate::ingest::DataIngester`] returns. This powers
//!      `onto_sql_ingest` / `sql-ingest` and lets the rest of the pipeline
//!      (mapping → N-Triples → SHACL → reason) stay unchanged.
//!
//! DuckDB is intentionally placed alongside Postgres rather than used as a
//! SPARQL parser. It is a *data integration backbone*: DuckDB's extensions
//! (httpfs, parquet, csv, json, postgres_scanner, sqlite_scanner, iceberg,
//! delta, …) let one SQL query federate over remote files, object stores,
//! and other databases — all of which then flow into RDF through the same
//! mapping layer used for CSV/Parquet/XLSX inputs today.

use anyhow::{anyhow, Result};
use std::collections::HashMap;

use crate::schema::TableInfo;

/// Recognised SQL backbone drivers.
///
/// # Examples
///
/// ```
/// use open_ontologies::sqlsource::SqlDriver;
///
/// // Variants can be compared for equality.
/// assert_eq!(SqlDriver::Postgres, SqlDriver::Postgres);
/// assert_eq!(SqlDriver::DuckDb,   SqlDriver::DuckDb);
/// assert_ne!(SqlDriver::Postgres, SqlDriver::DuckDb);
///
/// // Clone produces an independent copy with the same value.
/// let original = SqlDriver::DuckDb;
/// let cloned   = original;
/// assert_eq!(original, cloned);
/// ```
///
/// ## Mapping connection strings to drivers
///
/// ```
/// use open_ontologies::sqlsource::{detect_driver, SqlDriver};
///
/// // The driver enum is returned by detect_driver for further dispatch.
/// let driver = detect_driver("postgres://user:pass@localhost/mydb").unwrap();
/// assert_eq!(driver, SqlDriver::Postgres);
/// assert_eq!(driver.as_str(), "postgres");
///
/// let duck = detect_driver(":memory:").unwrap();
/// assert_eq!(duck, SqlDriver::DuckDb);
/// assert_eq!(duck.as_str(), "duckdb");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlDriver {
    Postgres,
    DuckDb,
}

impl SqlDriver {
    /// Return the canonical lowercase driver name as a `&'static str`.
    ///
    /// # Auto-instinct: driver names are lowercase, non-empty, and distinct
    ///
    /// All driver names are lowercase ASCII (no uppercase, no whitespace) so
    /// they can be embedded in connection-string prefixes, OCEL event attributes,
    /// and log records without escaping.
    ///
    /// ```
    /// use open_ontologies::sqlsource::SqlDriver;
    ///
    /// assert_eq!(SqlDriver::Postgres.as_str(), "postgres");
    /// assert_eq!(SqlDriver::DuckDb.as_str(), "duckdb");
    ///
    /// // Auto-instinct: names are lowercase, non-empty, and distinct.
    /// for driver in [SqlDriver::Postgres, SqlDriver::DuckDb] {
    ///     let name = driver.as_str();
    ///     assert!(!name.is_empty(), "driver name must be non-empty");
    ///     assert_eq!(name, name.to_lowercase(), "driver name must be lowercase");
    /// }
    /// assert_ne!(SqlDriver::Postgres.as_str(), SqlDriver::DuckDb.as_str(),
    ///            "driver names must be distinct");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            SqlDriver::Postgres => "postgres",
            SqlDriver::DuckDb => "duckdb",
        }
    }
}

/// Detect which SQL driver should handle a given connection string.
///
/// Recognised forms:
/// * `postgres://…` / `postgresql://…` → Postgres
/// * `duckdb://…` / `duckdb:…` → DuckDB
/// * `:memory:` → DuckDB (in-memory)
/// * `*.duckdb`, `*.ddb` file path → DuckDB
///
/// # Examples
///
/// ```
/// use open_ontologies::sqlsource::{detect_driver, SqlDriver};
///
/// // Postgres schemes
/// assert_eq!(detect_driver("postgres://user:pass@host/db").unwrap(), SqlDriver::Postgres);
/// assert_eq!(detect_driver("postgresql://user@host/db").unwrap(), SqlDriver::Postgres);
///
/// // DuckDB — in-memory sentinel
/// assert_eq!(detect_driver(":memory:").unwrap(), SqlDriver::DuckDb);
///
/// // DuckDB — URL schemes
/// assert_eq!(detect_driver("duckdb:///tmp/warehouse.db").unwrap(), SqlDriver::DuckDb);
/// assert_eq!(detect_driver("duckdb:/tmp/warehouse.db").unwrap(), SqlDriver::DuckDb);
///
/// // DuckDB — file extensions
/// assert_eq!(detect_driver("/data/store.duckdb").unwrap(), SqlDriver::DuckDb);
/// assert_eq!(detect_driver("./local.ddb").unwrap(), SqlDriver::DuckDb);
///
/// // Case-insensitive
/// assert_eq!(detect_driver("POSTGRES://U@H/D").unwrap(), SqlDriver::Postgres);
///
/// // Unknown scheme → error
/// assert!(detect_driver("mysql://host/db").is_err());
/// assert!(detect_driver("").is_err());
/// ```
pub fn detect_driver(connection: &str) -> Result<SqlDriver> {
    let trimmed = connection.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("empty connection string"));
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("postgres://") || lower.starts_with("postgresql://") {
        return Ok(SqlDriver::Postgres);
    }
    if lower.starts_with("duckdb://") || lower.starts_with("duckdb:") || trimmed == ":memory:" {
        return Ok(SqlDriver::DuckDb);
    }
    if lower.ends_with(".duckdb") || lower.ends_with(".ddb") {
        return Ok(SqlDriver::DuckDb);
    }
    Err(anyhow!(
        "unrecognised SQL connection string '{}': expected one of postgres://, postgresql://, duckdb://, :memory:, or a *.duckdb file path",
        trimmed
    ))
}

/// Strip the `duckdb:` / `duckdb://` prefix. Returns `:memory:` unchanged and
/// preserves bare file paths. The remaining string is what the duckdb crate
/// expects (a filesystem path or `:memory:`).
///
/// # Examples
///
/// ```
/// use open_ontologies::sqlsource::duckdb_target;
///
/// // Three-slash URL: strip prefix, keep path
/// assert_eq!(duckdb_target("duckdb:///tmp/x.db"), "/tmp/x.db");
///
/// // Two-slash URL: strip prefix, keep path
/// assert_eq!(duckdb_target("duckdb:/tmp/x.db"), "/tmp/x.db");
///
/// // Empty URL body → in-memory sentinel
/// assert_eq!(duckdb_target("duckdb://"), ":memory:");
/// assert_eq!(duckdb_target("duckdb:"), ":memory:");
///
/// // Already a bare sentinel — returned unchanged
/// assert_eq!(duckdb_target(":memory:"), ":memory:");
///
/// // Bare file path — returned unchanged
/// assert_eq!(duckdb_target("/data/warehouse.duckdb"), "/data/warehouse.duckdb");
/// ```
pub fn duckdb_target(connection: &str) -> String {
    let trimmed = connection.trim();
    if let Some(rest) = trimmed.strip_prefix("duckdb://") {
        if rest.is_empty() {
            return ":memory:".to_string();
        }
        return rest.to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("duckdb:") {
        if rest.is_empty() {
            return ":memory:".to_string();
        }
        return rest.to_string();
    }
    trimmed.to_string()
}

/// Run a SELECT query against the SQL backbone and return rows as
/// `Vec<HashMap<String,String>>` (the same shape as
/// [`crate::ingest::DataIngester::parse_file`]).
///
/// Compatible with [`crate::mapping::MappingConfig::rows_to_ntriples`].
///
/// # Auto-instinct: connection string is dispatched before any I/O
///
/// An unrecognised connection string is rejected synchronously — the driver
/// detection runs before any network or filesystem I/O, so bad strings fail
/// immediately without touching external resources.
///
/// ```
/// use open_ontologies::sqlsource::detect_driver;
///
/// // Auto-instinct: detect_driver is the same gate query_rows passes through.
/// // Unknown scheme → error before any I/O.
/// assert!(detect_driver("sqlite:///tmp/x.db").is_err());
/// assert!(detect_driver("mongodb://host/db").is_err());
///
/// // Known schemes succeed without network access (driver detection is pure).
/// assert!(detect_driver("postgres://user@host/db").is_ok());
/// assert!(detect_driver(":memory:").is_ok());
/// ```
///
/// # Examples (live I/O — not run in doctests)
///
/// ```no_run
/// # async fn example() -> anyhow::Result<()> {
/// use open_ontologies::sqlsource::query_rows;
///
/// // DuckDB in-memory: SELECT returns rows as HashMap<String,String>.
/// let rows = query_rows(":memory:", "SELECT 42 AS answer").await?;
/// assert_eq!(rows.len(), 1);
/// assert_eq!(rows[0]["answer"], "42");
/// # Ok(())
/// # }
/// ```
pub async fn query_rows(connection: &str, sql: &str) -> Result<Vec<HashMap<String, String>>> {
    match detect_driver(connection)? {
        SqlDriver::Postgres => query_postgres(connection, sql).await,
        SqlDriver::DuckDb => query_duckdb(connection, sql).await,
    }
}

/// Introspect tables/columns/foreign keys from the SQL backbone.
///
/// Used by `onto_import_schema` / `import-schema` to build OWL from a
/// relational schema. Both Postgres and DuckDB expose the SQL-standard
/// `information_schema.*` views that the introspectors rely on.
///
/// # Auto-instinct: feature-flag gate is enforced at runtime
///
/// Requesting a driver whose feature flag was not compiled in returns an `Err`
/// rather than silently producing empty results. The error message names the
/// missing feature so operators can diagnose mis-configured deployments.
///
/// ```
/// use open_ontologies::sqlsource::detect_driver;
/// use open_ontologies::sqlsource::SqlDriver;
///
/// // detect_driver confirms the driver *before* introspect_tables opens any
/// // connection. The same detection logic is reused by introspect_tables, so
/// // an unsupported connection string is rejected at the driver-detect layer.
/// let result = detect_driver("mysql://host/db");
/// assert!(result.is_err());
/// let msg = result.unwrap_err().to_string();
/// assert!(msg.contains("unrecognised SQL connection string"));
/// ```
///
/// # Examples (live I/O — not run in doctests)
///
/// ```no_run
/// # async fn example() -> anyhow::Result<()> {
/// use open_ontologies::sqlsource::introspect_tables;
///
/// // Requires the `duckdb` feature; queries information_schema.columns.
/// let tables = introspect_tables(":memory:").await?;
/// // An empty DuckDB in-memory DB has no user tables.
/// assert!(tables.is_empty());
/// # Ok(())
/// # }
/// ```
pub async fn introspect_tables(connection: &str) -> Result<Vec<TableInfo>> {
    match detect_driver(connection)? {
        SqlDriver::Postgres => {
            #[cfg(feature = "postgres")]
            {
                crate::schema::SchemaIntrospector::introspect_postgres(connection).await
            }
            #[cfg(not(feature = "postgres"))]
            {
                let _ = connection;
                Err(anyhow!(
                    "Postgres connection requested but the 'postgres' feature was not compiled in"
                ))
            }
        }
        SqlDriver::DuckDb => {
            #[cfg(feature = "duckdb")]
            {
                crate::schema::SchemaIntrospector::introspect_duckdb(&duckdb_target(connection))
            }
            #[cfg(not(feature = "duckdb"))]
            {
                let _ = connection;
                Err(anyhow!(
                    "DuckDB connection requested but the 'duckdb' feature was not compiled in"
                ))
            }
        }
    }
}

// ─── Postgres ────────────────────────────────────────────────────────────────

#[cfg(feature = "postgres")]
async fn query_postgres(connection: &str, sql: &str) -> Result<Vec<HashMap<String, String>>> {
    use sqlx::postgres::PgPoolOptions;
    use sqlx::Column;
    use sqlx::Row;
    use sqlx::TypeInfo;

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(connection)
        .await?;

    let pg_rows = sqlx::query(sql).fetch_all(&pool).await?;
    let mut rows = Vec::with_capacity(pg_rows.len());
    for row in &pg_rows {
        let mut map = HashMap::new();
        for (i, col) in row.columns().iter().enumerate() {
            let name = col.name().to_string();
            let value = pg_value_to_string(row, i, col.type_info().name());
            map.insert(name, value);
        }
        rows.push(map);
    }
    pool.close().await;
    Ok(rows)
}

#[cfg(not(feature = "postgres"))]
async fn query_postgres(_connection: &str, _sql: &str) -> Result<Vec<HashMap<String, String>>> {
    Err(anyhow!(
        "Postgres connection requested but the 'postgres' feature was not compiled in"
    ))
}

#[cfg(feature = "postgres")]
fn pg_value_to_string(row: &sqlx::postgres::PgRow, idx: usize, type_name: &str) -> String {
    use sqlx::Row;
    // Try common types in order; fall back to NULL → "" or a debug stringification.
    let upper = type_name.to_ascii_uppercase();
    macro_rules! try_get {
        ($t:ty) => {
            if let Ok(v) = row.try_get::<Option<$t>, _>(idx) {
                return v.map(|x| x.to_string()).unwrap_or_default();
            }
        };
    }
    match upper.as_str() {
        "BOOL" => try_get!(bool),
        "INT2" => try_get!(i16),
        "INT4" => try_get!(i32),
        "INT8" => try_get!(i64),
        "FLOAT4" => try_get!(f32),
        "FLOAT8" => try_get!(f64),
        "NUMERIC" => {
            // sqlx::types::BigDecimal isn't enabled — fall through to string.
        }
        "TEXT" | "VARCHAR" | "BPCHAR" | "NAME" | "CITEXT" | "UUID" => try_get!(String),
        "DATE" | "TIMESTAMP" | "TIMESTAMPTZ" | "TIME" | "TIMETZ" => try_get!(String),
        "JSON" | "JSONB" => {
            if let Ok(v) = row.try_get::<Option<serde_json::Value>, _>(idx) {
                return v.map(|x| x.to_string()).unwrap_or_default();
            }
        }
        _ => {}
    }
    // Generic last-resort: ask for a String.
    if let Ok(v) = row.try_get::<Option<String>, _>(idx) {
        return v.unwrap_or_default();
    }
    String::new()
}

// ─── DuckDB ──────────────────────────────────────────────────────────────────

#[cfg(feature = "duckdb")]
async fn query_duckdb(connection: &str, sql: &str) -> Result<Vec<HashMap<String, String>>> {
    let target = duckdb_target(connection);
    let sql = sql.to_string();
    tokio::task::spawn_blocking(move || query_duckdb_blocking(&target, &sql))
        .await
        .map_err(|e| anyhow!("duckdb worker panicked: {e}"))?
}

#[cfg(feature = "duckdb")]
fn query_duckdb_blocking(target: &str, sql: &str) -> Result<Vec<HashMap<String, String>>> {
    use duckdb::types::ValueRef;
    use duckdb::Connection;

    let conn = if target == ":memory:" {
        Connection::open_in_memory()?
    } else {
        Connection::open(target)?
    };

    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query([])?;

    // Column names are only available after query() has been called.
    let column_names: Vec<String> = match rows.as_ref() {
        Some(r) => r.column_names().iter().map(|s| s.to_string()).collect(),
        None => Vec::new(),
    };

    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let mut map = HashMap::new();
        for (i, name) in column_names.iter().enumerate() {
            let value = match row.get_ref(i)? {
                ValueRef::Null => String::new(),
                ValueRef::Boolean(b) => b.to_string(),
                ValueRef::TinyInt(v) => v.to_string(),
                ValueRef::SmallInt(v) => v.to_string(),
                ValueRef::Int(v) => v.to_string(),
                ValueRef::BigInt(v) => v.to_string(),
                ValueRef::HugeInt(v) => v.to_string(),
                ValueRef::UTinyInt(v) => v.to_string(),
                ValueRef::USmallInt(v) => v.to_string(),
                ValueRef::UInt(v) => v.to_string(),
                ValueRef::UBigInt(v) => v.to_string(),
                ValueRef::Float(v) => v.to_string(),
                ValueRef::Double(v) => v.to_string(),
                ValueRef::Decimal(v) => v.to_string(),
                ValueRef::Text(bytes) => String::from_utf8_lossy(bytes).to_string(),
                ValueRef::Blob(bytes) => format!("0x{}", hex_encode(bytes)),
                other => format!("{other:?}"),
            };
            map.insert(name.clone(), value);
        }
        out.push(map);
    }
    Ok(out)
}

#[cfg(feature = "duckdb")]
fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(not(feature = "duckdb"))]
async fn query_duckdb(_connection: &str, _sql: &str) -> Result<Vec<HashMap<String, String>>> {
    Err(anyhow!(
        "DuckDB connection requested but the 'duckdb' feature was not compiled in"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_driver_postgres() {
        assert_eq!(detect_driver("postgres://u:p@h/d").unwrap(), SqlDriver::Postgres);
        assert_eq!(detect_driver("postgresql://u@h/d").unwrap(), SqlDriver::Postgres);
        assert_eq!(detect_driver("POSTGRES://U@H/D").unwrap(), SqlDriver::Postgres);
    }

    #[test]
    fn detect_driver_duckdb() {
        assert_eq!(detect_driver("duckdb:///tmp/x.db").unwrap(), SqlDriver::DuckDb);
        assert_eq!(detect_driver("duckdb:/tmp/x.db").unwrap(), SqlDriver::DuckDb);
        assert_eq!(detect_driver(":memory:").unwrap(), SqlDriver::DuckDb);
        assert_eq!(detect_driver("/data/warehouse.duckdb").unwrap(), SqlDriver::DuckDb);
        assert_eq!(detect_driver("./shop.ddb").unwrap(), SqlDriver::DuckDb);
    }

    #[test]
    fn detect_driver_rejects_unknown() {
        assert!(detect_driver("").is_err());
        assert!(detect_driver("mysql://x").is_err());
        assert!(detect_driver("/data/file.csv").is_err());
    }

    #[test]
    fn duckdb_target_strips_prefix() {
        assert_eq!(duckdb_target("duckdb:///tmp/x.db"), "/tmp/x.db");
        assert_eq!(duckdb_target("duckdb:/tmp/x.db"), "/tmp/x.db");
        assert_eq!(duckdb_target("duckdb://"), ":memory:");
        assert_eq!(duckdb_target("duckdb:"), ":memory:");
        assert_eq!(duckdb_target(":memory:"), ":memory:");
        assert_eq!(duckdb_target("/data/warehouse.duckdb"), "/data/warehouse.duckdb");
    }
}
