use crate::config::AzureConfig;
use crate::error::{Result, ZelligError};
use crate::translator::Translator;
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;

#[derive(Deserialize)]
struct AzureItem {
    translations: Vec<AzureTranslation>,
}
#[derive(Deserialize)]
struct AzureTranslation {
    text: String,
}

const DEFAULT_ENDPOINT: &str = "https://api.cognitive.microsofttranslator.com";

pub struct AzureTranslator {
    client: reqwest::Client,
    api_key: String,
    region: String,
    endpoint: String,
}

impl AzureTranslator {
    pub fn new(config: &AzureConfig) -> Result<Self> {
        let api_key = config.api_key.clone().ok_or_else(|| {
            ZelligError::ConfigError("Azure Translator API key not configured".into())
        })?;
        let endpoint = config
            .endpoint
            .clone()
            .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string());
        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            region: config.region.clone(),
            endpoint: endpoint.trim_end_matches('/').to_string(),
        })
    }
}

impl Translator for AzureTranslator {
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
        let region = self.region.clone();
        let endpoint = self.endpoint.clone();
        Box::pin(async move {
            let url = {
                let mut url = reqwest::Url::parse(&format!("{}/translate", endpoint))
                    .unwrap_or_else(|_| {
                        reqwest::Url::parse(&format!("{}/translate", DEFAULT_ENDPOINT)).unwrap()
                    });
                {
                    let mut q = url.query_pairs_mut();
                    q.append_pair("api-version", "3.0");
                    q.append_pair("from", &source);
                    q.append_pair("to", &target);
                }
                url
            };

            let resp: reqwest::Response = client
                .post(url)
                .header("Ocp-Apim-Subscription-Key", &api_key)
                .header("Ocp-Apim-Subscription-Region", &region)
                .json(&[sonic_rs::json!({"text": text})])
                .send()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body: String = resp.text().await.unwrap_or_default();
                return Err(ZelligError::TranslationError(format!(
                    "Azure {}: {}",
                    status, body
                )));
            }

            let body: Vec<AzureItem> = resp
                .json()
                .await
                .map_err(|e: reqwest::Error| ZelligError::TranslationError(e.to_string()))?;

            body.into_iter()
                .next()
                .and_then(|mut item| item.translations.drain(..).next())
                .map(|t| t.text)
                .ok_or_else(|| ZelligError::TranslationError("invalid Azure response".into()))
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
        let source = source_lang.to_string();
        let target = target_lang.to_string();
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let region = self.region.clone();
        let endpoint = self.endpoint.clone();
        Box::pin(async move {
            #[derive(serde::Serialize)]
            struct AzureInput<'s> {
                text: &'s str,
            }
            let body: Vec<AzureInput<'_>> = texts.iter().map(|t| AzureInput { text: t }).collect();

            let url = {
                let mut url = reqwest::Url::parse(&format!("{}/translate", endpoint))
                    .unwrap_or_else(|_| {
                        reqwest::Url::parse(&format!("{}/translate", DEFAULT_ENDPOINT)).unwrap()
                    });
                {
                    let mut q = url.query_pairs_mut();
                    q.append_pair("api-version", "3.0");
                    q.append_pair("from", &source);
                    q.append_pair("to", &target);
                }
                url
            };

            let resp: reqwest::Response = client
                .post(url)
                .header("Ocp-Apim-Subscription-Key", &api_key)
                .header("Ocp-Apim-Subscription-Region", &region)
                .json(&body)
                .send()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body: String = resp.text().await.unwrap_or_default();
                return Err(ZelligError::TranslationError(format!(
                    "Azure {}: {}",
                    status, body
                )));
            }

            let body: Vec<AzureItem> = resp
                .json()
                .await
                .map_err(|e: reqwest::Error| ZelligError::TranslationError(e.to_string()))?;

            body.into_iter()
                .map(|mut item| {
                    item.translations
                        .drain(..)
                        .next()
                        .map(|t| t.text)
                        .ok_or_else(|| ZelligError::TranslationError("missing translation".into()))
                })
                .collect()
        })
    }
}
