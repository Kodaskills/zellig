use crate::config::Config;
use crate::error::Result;
use crate::translator::{Translator, TranslatorFactory};
use futures::future::try_join_all;

pub struct TranslationService {
    pub(crate) translator: Box<dyn Translator + Send + Sync>,
    config: Config,
}

impl TranslationService {
    pub async fn new(config: Config) -> Result<Self> {
        Ok(Self {
            translator: TranslatorFactory::new().create(&config).await?,
            config,
        })
    }

    pub async fn translate_text(
        &self,
        text: &str,
        source: &str,
        target: &str,
        ctx: Option<&str>,
    ) -> Result<String> {
        self.translator.translate(text, source, target, ctx).await
    }

    pub async fn batch_translate(
        &self,
        texts: &[String],
        source: &str,
        target: &str,
        ctx: Option<&str>,
    ) -> Result<Vec<String>> {
        try_join_all(
            texts
                .iter()
                .map(|t| self.translator.translate(t, source, target, ctx)),
        )
        .await
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
