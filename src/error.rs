use thiserror::Error;

/// Core error type for all ontology operations.
///
/// Every variant carries a human-readable message. The variants map 1-to-1
/// to the failure domains recognised by the Open Ontologies MCP server.
///
/// # Examples
///
/// ```
/// use open_ontologies::error::OntologyError;
///
/// // Construct each variant and verify its Display message.
/// let e = OntologyError::Parse("unexpected token '@'".into());
/// assert!(e.to_string().contains("parse error"));
///
/// let e = OntologyError::Validation("sh:minCount violated".into());
/// assert!(e.to_string().contains("validation error"));
///
/// let e = OntologyError::Sparql("syntax error near SELECT".into());
/// assert!(e.to_string().contains("SPARQL error"));
///
/// let e = OntologyError::Store("locked".into());
/// assert!(e.to_string().contains("store error"));
///
/// let e = OntologyError::Serialization("EOF".into());
/// assert!(e.to_string().contains("serialization error"));
///
/// let e = OntologyError::NotFound("urn:ex:Foo".into());
/// assert!(e.to_string().contains("not found"));
///
/// let e = OntologyError::Alignment("no candidates".into());
/// assert!(e.to_string().contains("alignment error"));
///
/// let e = OntologyError::Reasoning("unsatisfiable class".into());
/// assert!(e.to_string().contains("reasoning error"));
///
/// let e = OntologyError::Config("missing [general] section".into());
/// assert!(e.to_string().contains("configuration error"));
///
/// let e = OntologyError::FeatureDisabled("embeddings".into());
/// assert!(e.to_string().contains("feature not enabled"));
/// ```
#[derive(Error, Debug)]
pub enum OntologyError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("SPARQL error: {0}")]
    Sparql(String),

    #[error("store error: {0}")]
    Store(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("alignment error: {0}")]
    Alignment(String),

    #[error("reasoning error: {0}")]
    Reasoning(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("feature not enabled: {0}")]
    FeatureDisabled(String),
}

/// Convenience alias so callers can write `error::Result<T>` instead of
/// `std::result::Result<T, OntologyError>`.
///
/// # Examples
///
/// ```
/// use open_ontologies::error::{OntologyError, Result};
///
/// fn always_fails() -> Result<u32> {
///     Err(OntologyError::NotFound("demo".into()))
/// }
///
/// let err = always_fails().unwrap_err();
/// assert_eq!(err.to_string(), "not found: demo");
/// ```
pub type Result<T> = std::result::Result<T, OntologyError>;
