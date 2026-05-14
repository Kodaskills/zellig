use crate::config::BergamotConfig;
use crate::error::{Result, ZelligError};
use crate::translator::Translator;
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;

#[derive(Deserialize)]
struct BergamotResponse {
    translated: String,
}

/// Bergamot-compatible REST API (self-hosted Firefox Translations server).
/// Run via: docker run -p 8080:8080 ghcr.io/mozilla/firefox-translations:latest
pub struct BergamotTranslator {
    client: reqwest::Client,
    base_url: String,
}

impl BergamotTranslator {
    pub fn new(config: &BergamotConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: config.url.trim_end_matches('/').to_string(),
        }
    }
}

impl Translator for BergamotTranslator {
    fn translate<'a>(
        &'a self,
        text: &'a str,
        source_lang: &'a str,
        target_lang: &'a str,
        _context: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        let text = text.to_string();
        let source = source_lang.to_string();
        let target = target_lang.to_string();
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        Box::pin(async move {
            let resp = client
                .post(format!("{}/api/v1/translate", base_url))
                .json(&sonic_rs::json!({
                    "from": source,
                    "to": target,
                    "text": text,
                    "html": false,
                }))
                .send()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(ZelligError::TranslationError(format!(
                    "Bergamot {}: {}",
                    status, body
                )));
            }

            let body: BergamotResponse = resp
                .json()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            Ok(body.translated)
        })
    }
}
