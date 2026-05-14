use crate::config::LibreTranslateConfig;
use crate::error::{Result, ZelligError};
use crate::translator::Translator;
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;

#[derive(Deserialize)]
struct LibreResponse {
    #[serde(rename = "translatedText")]
    translated_text: Option<String>,
    error: Option<String>,
}

pub struct LibreTranslator {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

impl LibreTranslator {
    pub fn new(config: &LibreTranslateConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: config.url.trim_end_matches('/').to_string(),
            api_key: config.api_key.clone(),
        }
    }
}

impl Translator for LibreTranslator {
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
        let api_key = self.api_key.clone();
        Box::pin(async move {
            let mut body = sonic_rs::json!({
                "q": text,
                "source": source,
                "target": target,
                "format": "text",
            });
            if let Some(key) = api_key {
                body["api_key"] = sonic_rs::Value::from(key.as_str());
            }

            let resp = client
                .post(format!("{}/translate", base_url))
                .json(&body)
                .send()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(ZelligError::TranslationError(format!(
                    "LibreTranslate {}: {}",
                    status, body
                )));
            }

            let body: LibreResponse = resp
                .json()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            if let Some(err) = body.error {
                return Err(ZelligError::TranslationError(err));
            }

            body.translated_text.ok_or_else(|| {
                ZelligError::TranslationError("invalid LibreTranslate response".into())
            })
        })
    }
}
