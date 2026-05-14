use crate::config::YandexConfig;
use crate::error::{Result, ZelligError};
use crate::translator::Translator;
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;

#[derive(Deserialize)]
struct YandexResponse {
    translations: Vec<YandexTranslation>,
}
#[derive(Deserialize)]
struct YandexTranslation {
    text: String,
}

pub struct YandexTranslator {
    client: reqwest::Client,
    api_key: String,
    folder_id: Option<String>,
}

impl YandexTranslator {
    pub fn new(config: &YandexConfig) -> Result<Self> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| ZelligError::ConfigError("Yandex API key not configured".into()))?;
        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            folder_id: config.folder_id.clone(),
        })
    }
}

impl Translator for YandexTranslator {
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
        let api_key = self.api_key.clone();
        let folder_id = self.folder_id.clone();
        Box::pin(async move {
            let mut body = sonic_rs::json!({
                "sourceLanguageCode": source,
                "targetLanguageCode": target,
                "texts": [text],
            });
            if let Some(fid) = folder_id {
                body["folderId"] = sonic_rs::Value::from(fid.as_str());
            }

            let resp = client
                .post("https://translate.api.cloud.yandex.net/translate/v2/translate")
                .header("Authorization", format!("Api-Key {}", api_key))
                .json(&body)
                .send()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(ZelligError::TranslationError(format!(
                    "Yandex {}: {}",
                    status, body
                )));
            }

            let body: YandexResponse = resp
                .json()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            body.translations
                .into_iter()
                .next()
                .map(|t| t.text)
                .ok_or_else(|| ZelligError::TranslationError("empty Yandex response".into()))
        })
    }
}
