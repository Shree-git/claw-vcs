//! Three-way merge and revision integration helpers.
//!
//! The merge crate collects ancestry, groups changes, builds trees, emits merge
//! results, and reports conflicts without depending on CLI presentation.
//!
/// Ancestry helpers for revision graphs.
pub mod ancestor;
/// Collect revisions and changes for merge operations.
pub mod collect;
/// Emit merged objects and conflict records.
pub mod emit;
/// Merge error types.
pub mod error;
/// Group related patch operations.
pub mod group;
/// Rebase helpers.
pub mod rebase;
/// Build tree results from merge operations.
pub mod tree_build;

pub use error::MergeError;
