use crate::config::BaiduConfig;
use crate::error::{Result, ZelligError};
use crate::translator::Translator;
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
struct BaiduResponse {
    error_code: Option<String>,
    error_msg: Option<String>,
    trans_result: Option<Vec<BaiduTransResult>>,
}
#[derive(Deserialize)]
struct BaiduTransResult {
    dst: String,
}

pub struct BaiduTranslator {
    client: reqwest::Client,
    app_id: String,
    secret_key: String,
}

impl BaiduTranslator {
    pub fn new(config: &BaiduConfig) -> Result<Self> {
        let app_id = config
            .app_id
            .clone()
            .ok_or_else(|| ZelligError::ConfigError("Baidu app_id not configured".into()))?;
        let secret_key = config
            .secret_key
            .clone()
            .ok_or_else(|| ZelligError::ConfigError("Baidu secret_key not configured".into()))?;
        Ok(Self {
            client: reqwest::Client::new(),
            app_id,
            secret_key,
        })
    }
}

fn baidu_lang(code: &str) -> &str {
    match code {
        "zh" | "zh-hans" | "zh-cn" => "zh",
        "zh-tw" | "zh-hant" | "zh-hk" => "cht",
        "ja" => "jp",
        "ko" => "kor",
        "ar" => "ara",
        "fr" => "fra",
        "es" => "spa",
        _ => code,
    }
}

impl Translator for BaiduTranslator {
    fn translate<'a>(
        &'a self,
        text: &'a str,
        source_lang: &'a str,
        target_lang: &'a str,
        _context: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        let text = text.to_string();
        let from = baidu_lang(source_lang).to_string();
        let to = baidu_lang(target_lang).to_string();
        let client = self.client.clone();
        let app_id = self.app_id.clone();
        let secret_key = self.secret_key.clone();
        Box::pin(async move {
            let salt = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .to_string();
            let sign_input = format!("{}{}{}{}", app_id, text, salt, secret_key);
            let sign = format!("{:x}", md5::compute(sign_input.as_bytes()));

            let url = {
                let mut url =
                    reqwest::Url::parse("https://fanyi-api.baidu.com/api/trans/vip/translate")
                        .unwrap();
                {
                    let mut q = url.query_pairs_mut();
                    q.append_pair("appid", &app_id);
                    q.append_pair("q", &text);
                    q.append_pair("from", &from);
                    q.append_pair("to", &to);
                    q.append_pair("salt", &salt);
                    q.append_pair("sign", &sign);
                }
                url
            };

            let resp: reqwest::Response = client
                .get(url)
                .send()
                .await
                .map_err(|e: reqwest::Error| ZelligError::TranslationError(e.to_string()))?;

            let body: BaiduResponse = resp
                .json()
                .await
                .map_err(|e: reqwest::Error| ZelligError::TranslationError(e.to_string()))?;

            if let Some(code) = body.error_code {
                return Err(ZelligError::TranslationError(format!(
                    "Baidu error {}: {}",
                    code,
                    body.error_msg.as_deref().unwrap_or("unknown")
                )));
            }

            body.trans_result
                .and_then(|mut r| r.drain(..).next())
                .map(|t| t.dst)
                .ok_or_else(|| ZelligError::TranslationError("invalid Baidu response".into()))
        })
    }
}
