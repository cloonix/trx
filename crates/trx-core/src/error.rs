//! Error types for trx

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Issue not found: {0}")]
    NotFound(String),

    #[error("Issue already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid issue ID: {0}")]
    InvalidId(String),

    #[error("Dependency cycle detected: {0}")]
    CycleDetected(String),

    #[error("Store not initialized. Run 'trx init' first.")]
    NotInitialized,

    #[error("Store already initialized at {0}")]
    AlreadyInitialized(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid status: {0}")]
    InvalidStatus(String),

    #[error("Invalid issue type: {0}")]
    InvalidType(String),

    #[error("Service error: {0}")]
    Service(String),

    #[error("{0}")]
    Other(String),
}
