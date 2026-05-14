#[cfg(feature = "local")]
use crate::error::{Result, ZelligError};
#[cfg(feature = "local")]
use async_trait::async_trait;

#[cfg(feature = "local")]
pub struct WhatLangDetector;

#[cfg(feature = "local")]
impl WhatLangDetector {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "local")]
impl Default for WhatLangDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "local")]
#[async_trait]
impl super::LanguageDetector for WhatLangDetector {
    async fn detect_language(&self, text: &str) -> Result<super::LanguageInfo> {
        let info = whatlang::detect(text)
            .ok_or_else(|| ZelligError::DetectionError("Could not detect language".to_string()))?;

        Ok(super::LanguageInfo {
            code_iso639_1: Some(info.lang().code().to_string()),
            code_iso639_3: None, // whatlang doesn't provide ISO 639-3
            name: Some(info.lang().eng_name().to_string()),
            script: Some(info.script().name().to_string()),
            confidence: Some(info.confidence()),
            is_reliable: Some(info.is_reliable()),
        })
    }
}
