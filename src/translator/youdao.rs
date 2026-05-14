use crate::config::YoudaoConfig;
use crate::error::{Result, ZelligError};
use crate::translator::Translator;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::future::Future;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
struct YoudaoResponse {
    #[serde(rename = "errorCode")]
    error_code: String,
    translation: Option<Vec<String>>,
}

pub struct YoudaoTranslator {
    client: reqwest::Client,
    app_key: String,
    app_secret: String,
}

impl YoudaoTranslator {
    pub fn new(config: &YoudaoConfig) -> Result<Self> {
        let app_key = config
            .app_key
            .clone()
            .ok_or_else(|| ZelligError::ConfigError("Youdao app_key not configured".into()))?;
        let app_secret = config
            .app_secret
            .clone()
            .ok_or_else(|| ZelligError::ConfigError("Youdao app_secret not configured".into()))?;
        Ok(Self {
            client: reqwest::Client::new(),
            app_key,
            app_secret,
        })
    }
}

fn youdao_lang(code: &str) -> &str {
    match code.to_lowercase().as_str() {
        "zh" | "zh-hans" | "zh-cn" => "zh-CHS",
        "zh-tw" | "zh-hant" | "zh-hk" => "zh-CHT",
        _ => code,
    }
}

fn truncate_for_sign(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len <= 20 {
        text.to_string()
    } else {
        let first: String = chars[..10].iter().collect();
        let last: String = chars[len - 10..].iter().collect();
        format!("{}{}{}", first, len, last)
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

impl Translator for YoudaoTranslator {
    fn translate<'a>(
        &'a self,
        text: &'a str,
        source_lang: &'a str,
        target_lang: &'a str,
        _context: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        let text = text.to_string();
        let from = youdao_lang(source_lang).to_string();
        let to = youdao_lang(target_lang).to_string();
        let client = self.client.clone();
        let app_key = self.app_key.clone();
        let app_secret = self.app_secret.clone();
        Box::pin(async move {
            let curtime = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string();
            let salt = format!("{}_salt", curtime);
            let truncated = truncate_for_sign(&text);
            let sign_str = format!("{}{}{}{}{}", app_key, truncated, salt, curtime, app_secret);
            let sign = sha256_hex(&sign_str);

            let mut form_data = std::collections::HashMap::new();
            form_data.insert("q", text.clone());
            form_data.insert("from", from.clone());
            form_data.insert("to", to.clone());
            form_data.insert("appKey", app_key.clone());
            form_data.insert("salt", salt.clone());
            form_data.insert("sign", sign.clone());
            form_data.insert("signType", "v3".to_string());
            form_data.insert("curtime", curtime.clone());

            let resp: reqwest::Response = client
                .post("https://openapi.youdao.com/api")
                .form(&form_data)
                .send()
                .await
                .map_err(|e: reqwest::Error| ZelligError::TranslationError(e.to_string()))?;

            let body: YoudaoResponse = resp
                .json()
                .await
                .map_err(|e: reqwest::Error| ZelligError::TranslationError(e.to_string()))?;

            if body.error_code != "0" {
                return Err(ZelligError::TranslationError(format!(
                    "Youdao error code: {}",
                    body.error_code
                )));
            }

            body.translation
                .and_then(|mut t| t.drain(..).next())
                .ok_or_else(|| ZelligError::TranslationError("invalid Youdao response".into()))
        })
    }
}
