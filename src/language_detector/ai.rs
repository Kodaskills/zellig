use crate::error::{Result, ZelligError};
use async_trait::async_trait;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AiLanguageResponse {
    iso639_1: Option<String>,
    iso639_3: Option<String>,
    name: Option<String>,
    script: Option<String>,
    confidence: Option<f64>,
}

pub struct AiLanguageDetector {
    client: genai::Client,
    model: String,
}

impl AiLanguageDetector {
    pub fn new(model: &str) -> Self {
        Self {
            client: genai::Client::builder().build(),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl super::LanguageDetector for AiLanguageDetector {
    async fn detect_language(&self, text: &str) -> Result<super::LanguageInfo> {
        let system_msg = r#"You are a language detection tool. Return a JSON object with:
- iso639_1: ISO 639-1 code (e.g., "en", "fr", "ko") or null
- iso639_3: ISO 639-3 code (e.g., "eng", "fra", "kor") or null  
- name: English name (e.g., "English", "French", "Korean") or null
- script: Script name (e.g., "Latin", "Cyrillic", "Hangul") or null
- confidence: number between 0 and 1 or null

Respond ONLY with the JSON object, no markdown or explanations."#;

        let response = self
            .client
            .exec_chat(
                &self.model,
                genai::chat::ChatRequest::new(vec![
                    genai::chat::ChatMessage::system(system_msg),
                    genai::chat::ChatMessage::user(text),
                ]),
                None,
            )
            .await
            .map_err(|e| ZelligError::AiError(e.to_string()))?;

        let response_text = response
            .first_text()
            .ok_or_else(|| ZelligError::AiError("Empty response".to_string()))?;

        let parsed: AiLanguageResponse = sonic_rs::from_str(response_text)
            .map_err(|e| ZelligError::AiError(format!("Failed to parse JSON: {}", e)))?;

        Ok(super::LanguageInfo {
            code_iso639_1: parsed.iso639_1,
            code_iso639_3: parsed.iso639_3,
            name: parsed.name,
            script: parsed.script,
            confidence: parsed.confidence,
            is_reliable: None, // AI doesn't provide reliability info
        })
    }
}
