use std::collections::HashMap;
use std::sync::Arc;

use crate::codec::Codec;
use crate::PatchError;

/// Registry for resolving codecs by stable id, file extension, or fallback.
pub struct CodecRegistry {
    codecs: HashMap<String, Arc<dyn Codec>>,
    extension_map: HashMap<String, String>,
    fallback: Option<Arc<dyn Codec>>,
}

impl CodecRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            codecs: HashMap::new(),
            extension_map: HashMap::new(),
            fallback: None,
        }
    }

    /// Register a codec and map the provided extensions to its id.
    pub fn register(&mut self, codec: Arc<dyn Codec>, extensions: &[&str]) {
        let id = codec.id().to_string();
        for ext in extensions {
            self.extension_map.insert(ext.to_string(), id.clone());
        }
        self.codecs.insert(id, codec);
    }

    /// Register the fallback codec used when no extension-specific codec matches.
    pub fn set_fallback(&mut self, codec: Arc<dyn Codec>) {
        let id = codec.id().to_string();
        self.codecs.insert(id, codec.clone());
        self.fallback = Some(codec);
    }

    /// Get a codec by stable id.
    pub fn get(&self, codec_id: &str) -> Result<&Arc<dyn Codec>, PatchError> {
        self.codecs
            .get(codec_id)
            .ok_or_else(|| PatchError::CodecNotFound(codec_id.to_string()))
    }

    /// Get a codec by file extension without the leading dot.
    pub fn get_by_extension(&self, ext: &str) -> Option<&Arc<dyn Codec>> {
        let codec_id = self.extension_map.get(ext)?;
        self.codecs.get(codec_id)
    }

    /// Get the best codec for a file path, falling back when configured.
    pub fn get_for_path(&self, path: &str) -> Option<&Arc<dyn Codec>> {
        let ext = path.rsplit('.').next().unwrap_or("");
        self.get_by_extension(ext).or(self.fallback.as_ref())
    }

    /// Build the default registry with text, JSON, and binary codecs.
    pub fn default_registry() -> Self {
        use crate::binary::BinaryCodec;
        use crate::json_tree::JsonTreeCodec;
        use crate::text_line::TextLineCodec;

        let mut reg = Self::new();
        reg.register(
            Arc::new(TextLineCodec),
            &[
                "txt", "md", "rs", "py", "js", "ts", "c", "h", "cpp", "go", "rb", "sh", "toml",
                "yaml", "yml",
            ],
        );
        reg.register(Arc::new(JsonTreeCodec), &["json"]);
        reg.set_fallback(Arc::new(BinaryCodec));
        reg
    }
}

impl Default for CodecRegistry {
    fn default() -> Self {
        Self::default_registry()
    }
}
