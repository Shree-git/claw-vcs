use serde::{Deserialize, Serialize};

use crate::id::ObjectId;
use crate::CoreError;

/// File kind and permission mode for a tree entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileMode {
    /// Regular non-executable file.
    Regular,
    /// Executable file.
    Executable,
    /// Symbolic link.
    Symlink,
    /// Directory tree entry.
    Directory,
}

/// A named child entry inside a tree object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    /// Basename of the child entry.
    pub name: String,
    /// File mode for the child.
    pub mode: FileMode,
    /// Object ID referenced by the child.
    pub object_id: ObjectId,
}

/// Directory tree object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    /// Child entries sorted and validated by callers before persistence.
    pub entries: Vec<TreeEntry>,
}

/// Validate a tree entry basename for canonical object storage.
pub fn validate_tree_entry_name(name: &str) -> Result<(), CoreError> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        return Err(CoreError::InvalidTreeEntryName(name.to_string()));
    }

    if name.chars().any(char::is_control) {
        return Err(CoreError::InvalidTreeEntryName(name.to_string()));
    }

    Ok(())
}

impl Tree {
    /// Validate entry names and reject duplicate basenames.
    pub fn validate(&self) -> Result<(), CoreError> {
        let mut seen = std::collections::HashSet::with_capacity(self.entries.len());
        for entry in &self.entries {
            validate_tree_entry_name(&entry.name)?;
            if !seen.insert(entry.name.as_str()) {
                return Err(CoreError::Deserialization(format!(
                    "duplicate tree entry name: {}",
                    entry.name
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::validate_tree_entry_name;

    #[test]
    fn rejects_invalid_tree_entry_names() {
        assert!(validate_tree_entry_name("").is_err());
        assert!(validate_tree_entry_name("..").is_err());
        assert!(validate_tree_entry_name("a/b").is_err());
        assert!(validate_tree_entry_name("a\\b").is_err());
    }

    #[test]
    fn accepts_normal_tree_entry_names() {
        assert!(validate_tree_entry_name("README.md").is_ok());
        assert!(validate_tree_entry_name(".env.example").is_ok());
        assert!(validate_tree_entry_name("src").is_ok());
    }
}
