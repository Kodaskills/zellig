use crate::config::{Config, TranslationMode};
use crate::error::{Result, ZelligError};
use async_trait::async_trait;
use futures::future::try_join_all;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub mod ai;
pub mod azure;
pub mod baidu;
pub mod bergamot;
pub mod deepl;
pub mod google;
pub mod libre;
pub mod lingva;
pub mod local;
pub mod qq;
pub mod yandex;
pub mod youdao;

pub use ai::AiTranslator;
pub use azure::AzureTranslator;
pub use baidu::BaiduTranslator;
pub use bergamot::BergamotTranslator;
pub use deepl::DeepLTranslator;
pub use google::GoogleTranslator;
pub use libre::LibreTranslator;
pub use lingva::LingvaTranslator;
pub use local::LocalTranslator;
pub use qq::QqTranslator;
pub use yandex::YandexTranslator;
pub use youdao::YoudaoTranslator;

pub(crate) trait LocalBackend: Send + Sync {
    fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String>;
    fn device_label(&self) -> &str { "cpu" }

    fn batch_translate(
        &self,
        texts: &[String],
        source_lang: &str,
        target_lang: &str,
    ) -> Result<Vec<String>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.translate(text, source_lang, target_lang)?);
        }
        Ok(results)
    }
}

pub trait Translator: Send + Sync {
    fn device_label(&self) -> &str { "" }

    fn translate<'a>(
        &'a self,
        text: &'a str,
        source_lang: &'a str,
        target_lang: &'a str,
        context: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>;

    fn batch_translate<'a>(
        &'a self,
        texts: &'a [String],
        source_lang: &'a str,
        target_lang: &'a str,
        context: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>>> + Send + 'a>> {
        Box::pin(async move {
            let sem = Arc::new(Semaphore::new(8));
            let futs = texts.iter().map(|t| {
                let sem = Arc::clone(&sem);
                async move {
                    let _permit = sem.acquire_owned().await.expect("semaphore closed");
                    self.translate(t, source_lang, target_lang, context).await
                }
            });
            try_join_all(futs).await
        })
    }
}

pub struct PromptBuilder;
impl PromptBuilder {
    pub fn build_system(source_lang: &str, target_lang: &str, context: Option<&str>) -> String {
        match context {
            Some(ctx) => format!(
                "You are a professional translator. Translate from {} to {}. Context: {}. Provide only translated text.",
                source_lang, target_lang, ctx
            ),
            None => format!(
                "You are a professional translator. Translate from {} to {}. Provide only translated text.",
                source_lang, target_lang
            ),
        }
    }
}

#[async_trait]
pub trait TranslatorCreator: Send + Sync {
    async fn create(&self, config: &Config) -> Result<Box<dyn Translator>>;
}

pub struct TranslatorFactory {
    creators: HashMap<TranslationMode, Box<dyn TranslatorCreator>>,
}

impl Default for TranslatorFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl TranslatorFactory {
    pub fn new() -> Self {
        let mut creators: HashMap<TranslationMode, Box<dyn TranslatorCreator>> = HashMap::new();
        creators.insert(TranslationMode::Ai, Box::new(AiCreator));
        creators.insert(TranslationMode::Google, Box::new(GoogleCreator));
        creators.insert(TranslationMode::DeepL, Box::new(DeepLCreator));
        creators.insert(TranslationMode::Yandex, Box::new(YandexCreator));
        creators.insert(TranslationMode::LibreTranslate, Box::new(LibreCreator));
        creators.insert(TranslationMode::Azure, Box::new(AzureCreator));
        creators.insert(TranslationMode::Bergamot, Box::new(BergamotCreator));
        creators.insert(TranslationMode::Baidu, Box::new(BaiduCreator));
        creators.insert(TranslationMode::Youdao, Box::new(YoudaoCreator));
        creators.insert(TranslationMode::Qq, Box::new(QqCreator));
        creators.insert(TranslationMode::Lingva, Box::new(LingvaCreator));
        #[cfg(feature = "local")]
        creators.insert(TranslationMode::Local, Box::new(LocalCreator));
        Self { creators }
    }

    pub async fn create(&self, config: &Config) -> Result<Box<dyn Translator>> {
        self.creators
            .get(&config.mode)
            .ok_or_else(|| {
                #[cfg(not(feature = "local"))]
                if config.mode == TranslationMode::Local {
                    return ZelligError::ConfigError(
                        "Local backend not available — binary compiled without 'local' feature"
                            .to_string(),
                    );
                }
                ZelligError::ConfigError(format!(
                    "No translator for {:?} — check feature flags or config",
                    config.mode
                ))
            })?
            .create(config)
            .await
    }

    pub fn register(&mut self, mode: TranslationMode, creator: Box<dyn TranslatorCreator>) {
        self.creators.insert(mode, creator);
    }
}

// ── Creators ────────────────────────────────────────────

struct AiCreator;
#[async_trait]
impl TranslatorCreator for AiCreator {
    async fn create(&self, config: &Config) -> Result<Box<dyn Translator>> {
        Ok(Box::new(AiTranslator::new(&config.ai)?))
    }
}

#[cfg(feature = "local")]
struct LocalCreator;
#[cfg(feature = "local")]
#[async_trait]
impl TranslatorCreator for LocalCreator {
    async fn create(&self, config: &Config) -> Result<Box<dyn Translator>> {
        Ok(Box::new(LocalTranslator::new(&config.local).await?))
    }
}

struct GoogleCreator;
#[async_trait]
impl TranslatorCreator for GoogleCreator {
    async fn create(&self, _config: &Config) -> Result<Box<dyn Translator>> {
        Ok(Box::new(GoogleTranslator::new()))
    }
}

struct DeepLCreator;
#[async_trait]
impl TranslatorCreator for DeepLCreator {
    async fn create(&self, config: &Config) -> Result<Box<dyn Translator>> {
        Ok(Box::new(DeepLTranslator::new(&config.deepl)?))
    }
}

struct YandexCreator;
#[async_trait]
impl TranslatorCreator for YandexCreator {
    async fn create(&self, config: &Config) -> Result<Box<dyn Translator>> {
        Ok(Box::new(YandexTranslator::new(&config.yandex)?))
    }
}

struct LibreCreator;
#[async_trait]
impl TranslatorCreator for LibreCreator {
    async fn create(&self, config: &Config) -> Result<Box<dyn Translator>> {
        Ok(Box::new(LibreTranslator::new(&config.libre_translate)))
    }
}

struct AzureCreator;
#[async_trait]
impl TranslatorCreator for AzureCreator {
    async fn create(&self, config: &Config) -> Result<Box<dyn Translator>> {
        Ok(Box::new(AzureTranslator::new(&config.azure)?))
    }
}

struct BergamotCreator;
#[async_trait]
impl TranslatorCreator for BergamotCreator {
    async fn create(&self, config: &Config) -> Result<Box<dyn Translator>> {
        Ok(Box::new(BergamotTranslator::new(&config.bergamot)))
    }
}

struct BaiduCreator;
#[async_trait]
impl TranslatorCreator for BaiduCreator {
    async fn create(&self, config: &Config) -> Result<Box<dyn Translator>> {
        Ok(Box::new(BaiduTranslator::new(&config.baidu)?))
    }
}

struct YoudaoCreator;
#[async_trait]
impl TranslatorCreator for YoudaoCreator {
    async fn create(&self, config: &Config) -> Result<Box<dyn Translator>> {
        Ok(Box::new(YoudaoTranslator::new(&config.youdao)?))
    }
}

struct QqCreator;
#[async_trait]
impl TranslatorCreator for QqCreator {
    async fn create(&self, config: &Config) -> Result<Box<dyn Translator>> {
        Ok(Box::new(QqTranslator::new(&config.qq)?))
    }
}

struct LingvaCreator;
#[async_trait]
impl TranslatorCreator for LingvaCreator {
    async fn create(&self, config: &Config) -> Result<Box<dyn Translator>> {
        Ok(Box::new(LingvaTranslator::new(&config.lingva)))
    }
}
