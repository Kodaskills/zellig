use crate::config::LingvaConfig;
use crate::error::{Result, ZelligError};
use crate::translator::Translator;
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;

#[derive(Deserialize)]
struct LingvaResponse {
    translation: String,
}

pub struct LingvaTranslator {
    client: reqwest::Client,
    instance_url: String,
}

impl LingvaTranslator {
    pub fn new(config: &LingvaConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            instance_url: config.instance_url.trim_end_matches('/').to_string(),
        }
    }
}

fn percent_encode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

impl Translator for LingvaTranslator {
    fn translate<'a>(
        &'a self,
        text: &'a str,
        source_lang: &'a str,
        target_lang: &'a str,
        _context: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        let encoded = percent_encode_path(text);
        let source = source_lang.to_string();
        let target = target_lang.to_string();
        let client = self.client.clone();
        let instance_url = self.instance_url.clone();
        Box::pin(async move {
            let url = format!("{}/api/v1/{}/{}/{}", instance_url, source, target, encoded);

            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            if !resp.status().is_success() {
                return Err(ZelligError::TranslationError(format!(
                    "Lingva HTTP {}",
                    resp.status()
                )));
            }

            let body: LingvaResponse = resp
                .json()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            Ok(body.translation)
        })
    }
}
