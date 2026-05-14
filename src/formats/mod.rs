use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

pub mod json;
pub mod markdown;
pub mod po;
pub mod toml;
pub mod xliff;
pub mod yaml;

pub use json::JsonHandler;
pub use markdown::MarkdownHandler;
pub use po::PoHandler;
pub use toml::TomlHandler;
pub use xliff::XliffHandler;
pub use yaml::YamlHandler;

#[async_trait]
pub trait FormatHandler: Send + Sync {
    async fn extract(&self, content: &str) -> crate::error::Result<Vec<TranslatableString>>;
    async fn reconstruct(
        &self,
        original: &str,
        translations: &[TranslatedString],
    ) -> crate::error::Result<String>;
}

#[derive(Debug, Clone)]
pub struct TranslatableString {
    pub id: String,
    pub text: String,
    pub _context: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TranslatedString {
    pub id: String,
    pub translated_text: String,
}

pub struct FormatRegistry {
    handlers: HashMap<String, Arc<dyn FormatHandler>>,
}

impl Default for FormatRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatRegistry {
    pub fn new() -> Self {
        let mut handlers: HashMap<String, Arc<dyn FormatHandler>> = HashMap::new();
        handlers.insert(
            "json".into(),
            Arc::new(JsonHandler::new()) as Arc<dyn FormatHandler>,
        );
        handlers.insert("yaml".into(), Arc::new(YamlHandler::new()));
        handlers.insert("yml".into(), Arc::new(YamlHandler::new()));
        handlers.insert("toml".into(), Arc::new(TomlHandler::new()));
        handlers.insert("md".into(), Arc::new(MarkdownHandler::new()));
        handlers.insert("markdown".into(), Arc::new(MarkdownHandler::new()));
        handlers.insert("po".into(), Arc::new(PoHandler::new()));
        handlers.insert("xlf".into(), Arc::new(XliffHandler::new()));
        handlers.insert("xliff".into(), Arc::new(XliffHandler::new()));
        Self { handlers }
    }

    pub fn get_handler(&self, path: &str) -> Option<Arc<dyn FormatHandler>> {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())?
            .to_lowercase();
        self.handlers.get(&ext).cloned()
    }
}

pub fn detect_format(path: &str) -> Option<Arc<dyn FormatHandler>> {
    static REGISTRY: std::sync::OnceLock<FormatRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(FormatRegistry::new).get_handler(path)
}
