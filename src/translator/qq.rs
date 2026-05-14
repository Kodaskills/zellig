use crate::config::QqConfig;
use crate::error::{Result, ZelligError};
use crate::translator::Translator;
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::future::Future;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
struct QqWrapper {
    #[serde(rename = "Response")]
    response: QqResponse,
}
#[derive(Deserialize)]
struct QqResponse {
    #[serde(rename = "TargetText")]
    target_text: Option<String>,
    #[serde(rename = "Error")]
    error: Option<QqError>,
}
#[derive(Deserialize)]
struct QqError {
    #[serde(rename = "Code")]
    code: String,
    #[serde(rename = "Message")]
    message: String,
}

type HmacSha256 = Hmac<Sha256>;

pub struct QqTranslator {
    client: reqwest::Client,
    secret_id: String,
    secret_key: String,
    region: String,
}

impl QqTranslator {
    pub fn new(config: &QqConfig) -> Result<Self> {
        let secret_id = config
            .secret_id
            .clone()
            .ok_or_else(|| ZelligError::ConfigError("QQ secret_id not configured".into()))?;
        let secret_key = config
            .secret_key
            .clone()
            .ok_or_else(|| ZelligError::ConfigError("QQ secret_key not configured".into()))?;
        Ok(Self {
            client: reqwest::Client::new(),
            secret_id,
            secret_key,
            region: config.region.clone(),
        })
    }
}

fn sha256_hex(input: &str) -> String {
    let mut h = Sha256::new();
    h.update(input.as_bytes());
    hex::encode(h.finalize())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn timestamp_to_date(ts: u64) -> String {
    // Manual Gregorian calculation — avoids chrono dependency
    let mut days = ts / 86400;
    let mut year = 1970u32;
    loop {
        let in_year = if is_leap(year) { 366 } else { 365 };
        if days < in_year {
            break;
        }
        days -= in_year;
        year += 1;
    }
    let months = if is_leap(year) {
        [31u64, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u32;
    for &m in &months {
        if days < m {
            break;
        }
        days -= m;
        month += 1;
    }
    format!("{:04}-{:02}-{:02}", year, month, days + 1)
}

fn is_leap(y: u32) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

fn tc3_authorization(
    secret_id: &str,
    secret_key: &str,
    payload: &str,
    timestamp: u64,
    region: &str,
) -> String {
    let date = timestamp_to_date(timestamp);
    let service = "tmt";
    let host = "tmt.tencentcloudapi.com";
    let content_type = "application/json; charset=utf-8";

    let canonical_headers = format!("content-type:{}\nhost:{}\n", content_type, host);
    let signed_headers = "content-type;host";
    let payload_hash = sha256_hex(payload);
    let canonical_request = format!(
        "POST\n/\n\n{}\n{}\n{}",
        canonical_headers, signed_headers, payload_hash
    );

    let credential_scope = format!("{}/{}/tc3_request", date, service);
    let string_to_sign = format!(
        "TC3-HMAC-SHA256\n{}\n{}\n{}",
        timestamp,
        credential_scope,
        sha256_hex(&canonical_request)
    );

    let secret_date = hmac_sha256(format!("TC3{}", secret_key).as_bytes(), date.as_bytes());
    let secret_service = hmac_sha256(&secret_date, service.as_bytes());
    let secret_signing = hmac_sha256(&secret_service, b"tc3_request");
    let signature = hex::encode(hmac_sha256(&secret_signing, string_to_sign.as_bytes()));

    let _ = region; // included in request header, not in auth
    format!(
        "TC3-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        secret_id, credential_scope, signed_headers, signature
    )
}

impl Translator for QqTranslator {
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
        let secret_id = self.secret_id.clone();
        let secret_key = self.secret_key.clone();
        let region = self.region.clone();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let payload = sonic_rs::json!({
            "SourceText": text,
            "Source": source,
            "Target": target,
            "ProjectId": 0,
        })
        .to_string();

        let authorization =
            tc3_authorization(&secret_id, &secret_key, &payload, timestamp, &region);

        Box::pin(async move {
            let resp = client
                .post("https://tmt.tencentcloudapi.com/")
                .header("Authorization", authorization)
                .header("Content-Type", "application/json; charset=utf-8")
                .header("Host", "tmt.tencentcloudapi.com")
                .header("X-TC-Action", "TextTranslate")
                .header("X-TC-Version", "2018-05-07")
                .header("X-TC-Timestamp", timestamp.to_string())
                .header("X-TC-Region", &region)
                .body(payload)
                .send()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            let body: QqWrapper = resp
                .json()
                .await
                .map_err(|e| ZelligError::TranslationError(e.to_string()))?;

            if let Some(err) = body.response.error {
                return Err(ZelligError::TranslationError(format!(
                    "QQ {}: {}",
                    err.code, err.message
                )));
            }

            body.response
                .target_text
                .ok_or_else(|| ZelligError::TranslationError("invalid QQ response".into()))
        })
    }
}
