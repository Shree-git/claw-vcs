//! Core Claw VCS object types, IDs, hashing, and COF encoding.
//!
//! `claw-core` owns the stable in-repository data model: typed objects,
//! content-addressed IDs, Protocol Buffer conversions, and Claw Object Format
//! helpers. Higher-level crates should use these APIs instead of duplicating
//! serialization or hashing behavior.
//!
//! # Example
//!
//! ```rust
//! use claw_core::hash::content_hash;
//! use claw_core::object::TypeTag;
//!
//! let id = content_hash(TypeTag::Blob, b"hello");
//! assert_eq!(id.as_bytes().len(), 32);
//! ```
//!
#![deny(missing_docs)]

/// Claw Object Format encoding and decoding.
pub mod cof;
/// Core error types shared across object encoding and conversion.
pub mod error;
/// Prost-generated Protocol Buffer bindings.
pub mod generated;
/// Domain-separated content hashing helpers.
pub mod hash;
/// Strongly typed object, intent, change, and conflict identifiers.
pub mod id;
/// Top-level repository object enum and type tags.
pub mod object;
/// Deterministic conversions between core objects and generated protobuf types.
pub mod proto_conv;
/// Hand-written Claw VCS object model types.
pub mod types;

pub use error::CoreError;
pub use hash::content_hash;
pub use id::{ChangeId, IntentId, ObjectId};
pub use object::Object;
