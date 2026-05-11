use claw_core::types::PatchOp;

use crate::PatchError;

/// A content-aware patch implementation.
///
/// Codecs define how to diff, apply, invert, commute, and three-way merge a
/// specific content family. Implementations should return typed errors instead
/// of panicking on malformed patch operations or invalid input bytes.
pub trait Codec: Send + Sync {
    /// Stable codec identifier stored on patch objects.
    fn id(&self) -> &str;

    /// Build patch operations that transform `old` bytes into `new` bytes.
    fn diff(&self, old: &[u8], new: &[u8]) -> Result<Vec<PatchOp>, PatchError>;

    /// Apply patch operations to `base` bytes.
    fn apply(&self, base: &[u8], ops: &[PatchOp]) -> Result<Vec<u8>, PatchError>;

    /// Return operations that undo the provided operations.
    fn invert(&self, ops: &[PatchOp]) -> Result<Vec<PatchOp>, PatchError>;

    /// Reorder non-conflicting operations so left and right can be applied in either order.
    fn commute(
        &self,
        left: &[PatchOp],
        right: &[PatchOp],
    ) -> Result<(Vec<PatchOp>, Vec<PatchOp>), PatchError>;

    /// Merge independently edited byte streams against a common base.
    fn merge3(&self, base: &[u8], left: &[u8], right: &[u8]) -> Result<Vec<u8>, PatchError>;
}
