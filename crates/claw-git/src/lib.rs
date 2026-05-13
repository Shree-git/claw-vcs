//! Git import and export bridge for Claw VCS.
//!
//! This crate translates between Git objects/refs and Claw repository objects.
//! It is an interop boundary and should be tested with real `git` commands
//! whenever behavior changes.
//!
/// Blob conversion between Git and Claw.
pub mod blob_convert;
/// Commit/revision conversion between Git and Claw.
pub mod commit_convert;
/// Git bridge error types.
pub mod error;
/// Export Claw repositories into Git repositories.
pub mod exporter;
/// Import Git repositories into Claw repositories.
pub mod importer;
/// Tree conversion between Git and Claw.
pub mod tree_convert;

pub use error::GitExportError;
pub use error::GitImportError;
