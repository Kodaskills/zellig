use crate::config::DeepLConfig;
use crate::error::{Result, ZelligError};
use crate::translator::Translator;
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;

#[derive(Deserialize)]
struct DeepLResponse {
    translations: Vec<DeepLTranslation>,
}
#[derive(Deserialize)]
struct DeepLTranslation {
    text: String,
}

pub struct DeepLTranslator {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl DeepLTranslator {
    pub fn new(config: &DeepLConfig) -> Result<Self> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| ZelligError::ConfigError("DeepL API key not configured".into()))?;
        let base_url = if config.pro {
            "https://api.deepl.com".to_string()
        } else {
            "https://api-free.deepl.com".to_string()
        };
        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            base_url,
        })
    }
}

fn deepl_source(code: &str) -> String {
    code.to_uppercase()
}

fn deepl_target(code: &str) -> String {
    match code.to_lowercase().as_str() {
        "en" => "EN-US".to_string(),
        "pt" => "PT-BR".to_string(),
        "zh" | "zh-hans" | "zh-cn" => "ZH-HANS".to_string(),
        "zh-tw" | "zh-hant" | "zh-hk" => "ZH-HANT".to_string(),
        _ => code.to_uppercase(),
    }
}

impl Translator for DeepLTranslator {
    fn translate<'a>(
        &'a self,
        text: &'a str,
        source_lang: &'a str,
        target_lang: &'a str,
        _context: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        let text = text.to_string();
        let source = deepl_source(source_lang);
        let target = deepl_target(target_lang);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        Box::pin(async move {
            let resp = client
                .post(format!("{}/v2/translate", base_url))
                .header("Authorization", format!("DeepL-Auth-Key {}", api_key))
                .json(&sonic_rs::json!({
                    "text": [text],
                    "source_lang": source,
                    "target_lang": target,
                }))
                .send()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(ZelligError::TranslationError(format!(
                    "DeepL {}: {}",
                    status, body
                )));
            }

            let body: DeepLResponse = resp
                .json()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            body.translations
                .into_iter()
                .next()
                .map(|t| t.text)
                .ok_or_else(|| ZelligError::TranslationError("empty DeepL response".into()))
        })
    }

    fn batch_translate<'a>(
        &'a self,
        texts: &'a [String],
        source_lang: &'a str,
        target_lang: &'a str,
        _context: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>>> + Send + 'a>> {
        let texts = texts.to_vec();
        let source = deepl_source(source_lang);
        let target = deepl_target(target_lang);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        Box::pin(async move {
            let resp = client
                .post(format!("{}/v2/translate", base_url))
                .header("Authorization", format!("DeepL-Auth-Key {}", api_key))
                .json(&sonic_rs::json!({
                    "text": texts,
                    "source_lang": source,
                    "target_lang": target,
                }))
                .send()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(ZelligError::TranslationError(format!(
                    "DeepL {}: {}",
                    status, body
                )));
            }

            let body: DeepLResponse = resp
                .json()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            Ok(body.translations.into_iter().map(|t| t.text).collect())
        })
    }
}
