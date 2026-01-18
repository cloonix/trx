//! trx-core: Core library for the trx issue tracker
//!
//! Provides the data model, storage, and graph operations for a minimal
//! git-backed issue tracker. Supports both JSONL (v1) and CRDT (v2) storage.

pub mod config;
pub mod crdt_store;
pub mod error;
pub mod graph;
pub mod id;
pub mod issue;
pub mod service;
pub mod store;
pub mod unified_store;

pub use config::{Config, StorageVersion};
pub use crdt_store::CrdtStore;
pub use error::Error;
pub use graph::IssueGraph;
pub use id::generate_id;
pub use issue::{Dependency, DependencyType, Issue, IssueType, Status};
pub use service::{ServiceManager, ServiceStatus};
pub use store::Store;
pub use unified_store::{MigrationResult, UnifiedStore, migrate_v1_to_v2, rollback_v2_to_v1};

/// Result type for trx operations
pub type Result<T> = std::result::Result<T, Error>;
