use crate::error::Result;
use async_trait::async_trait;

pub mod ai;
pub mod local;

pub use ai::AiLanguageDetector;
#[cfg(feature = "local")]
pub use local::WhatLangDetector;

#[derive(Debug, Clone)]
pub struct LanguageInfo {
    pub code_iso639_1: Option<String>, // "en", "fr", "ko"
    pub code_iso639_3: Option<String>, // "eng", "fra", "kor"
    pub name: Option<String>,          // "English", "French", "Korean"
    pub script: Option<String>,        // "Latin", "Cyrillic", "Hangul"
    pub confidence: Option<f64>,       // 0.0 - 1.0
    pub is_reliable: Option<bool>,
}

impl std::fmt::Display for LanguageInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Language Information:")?;
        if let Some(ref code) = self.code_iso639_1 {
            writeln!(f, "  ISO 639-1:  {}", code)?;
        }
        if let Some(ref code) = self.code_iso639_3 {
            writeln!(f, "  ISO 639-3:  {}", code)?;
        }
        if let Some(ref name) = self.name {
            writeln!(f, "  Name:        {}", name)?;
        }
        if let Some(ref script) = self.script {
            writeln!(f, "  Script:      {}", script)?;
        }
        if let Some(conf) = self.confidence {
            writeln!(f, "  Confidence:  {:.2}%", conf * 100.0)?;
        }
        if let Some(reliable) = self.is_reliable {
            writeln!(f, "  Reliable:    {}", reliable)?;
        }
        Ok(())
    }
}

#[async_trait]
pub trait LanguageDetector: Send + Sync {
    async fn detect_language(&self, text: &str) -> Result<LanguageInfo>;
}
