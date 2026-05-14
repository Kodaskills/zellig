use crate::error::{Result, ZelligError};
use crate::translator::Translator;
use sonic_rs::{JsonContainerTrait, JsonValueTrait};
use std::future::Future;
use std::pin::Pin;

pub struct GoogleTranslator {
    client: reqwest::Client,
}

impl GoogleTranslator {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for GoogleTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl Translator for GoogleTranslator {
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
        Box::pin(async move {
            let url = {
                let mut url =
                    reqwest::Url::parse("https://translate.googleapis.com/translate_a/single")
                        .unwrap();
                {
                    let mut q = url.query_pairs_mut();
                    q.append_pair("client", "gtx");
                    q.append_pair("sl", &source);
                    q.append_pair("tl", &target);
                    q.append_pair("dt", "t");
                    q.append_pair("q", &text);
                }
                url
            };

            let resp: reqwest::Response = client
                .get(url)
                .send()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            if !resp.status().is_success() {
                return Err(ZelligError::TranslationError(format!(
                    "HTTP {}",
                    resp.status()
                )));
            }

            let bytes = resp
                .bytes()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;
            let body: sonic_rs::Value = sonic_rs::from_slice(&bytes)
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            // Response: [[[translated, original, ...], ...], ...]
            let translated = body[0]
                .as_array()
                .ok_or_else(|| ZelligError::TranslationError("invalid Google response".into()))?
                .iter()
                .filter_map(|item: &sonic_rs::Value| item[0].as_str())
                .collect::<Vec<_>>()
                .join("");

            if translated.is_empty() {
                return Err(ZelligError::TranslationError("empty translation".into()));
            }
            Ok(translated)
        })
    }
}
