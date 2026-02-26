use serde::{Deserialize, Serialize};

use crate::id::ObjectId;
use crate::CoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileMode {
    Regular,
    Executable,
    Symlink,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub name: String,
    pub mode: FileMode,
    pub object_id: ObjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

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
