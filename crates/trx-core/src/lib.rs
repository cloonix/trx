//! trx-core: Core library for the trx issue tracker
//!
//! Provides the data model, storage, and graph operations for a minimal
//! git-backed issue tracker. No daemon, no SQLite - just JSONL files.

pub mod config;
pub mod error;
pub mod graph;
pub mod id;
pub mod issue;
pub mod service;
pub mod store;

pub use config::Config;
pub use error::Error;
pub use graph::IssueGraph;
pub use id::generate_id;
pub use issue::{Dependency, DependencyType, Issue, IssueType, Status};
pub use service::{ServiceManager, ServiceStatus};
pub use store::Store;

/// Result type for trx operations
pub type Result<T> = std::result::Result<T, Error>;
