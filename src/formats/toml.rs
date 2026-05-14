use async_trait::async_trait;
use std::collections::HashMap;

pub struct TomlHandler;

impl TomlHandler {
    pub fn new() -> Self {
        TomlHandler
    }
}

impl Default for TomlHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::FormatHandler for TomlHandler {
    async fn extract(&self, content: &str) -> crate::error::Result<Vec<super::TranslatableString>> {
        let value: serde_json::Value = toml::from_str(content)
            .map_err(|e| crate::error::ZelligError::ConfigError(e.to_string()))?;
        let mut strings = Vec::new();
        extract_from_toml(&value, &mut strings, "toml");
        Ok(strings)
    }

    async fn reconstruct(
        &self,
        original: &str,
        translations: &[super::TranslatedString],
    ) -> crate::error::Result<String> {
        let mut value: serde_json::Value = toml::from_str(original)
            .map_err(|e| crate::error::ZelligError::ConfigError(e.to_string()))?;
        let translation_map: HashMap<String, String> = translations
            .iter()
            .map(|t| (t.id.clone(), t.translated_text.clone()))
            .collect();
        apply_translations(&mut value, &translation_map);
        toml::to_string_pretty(&value)
            .map_err(|e| crate::error::ZelligError::ConfigError(e.to_string()))
    }
}

fn extract_from_toml(
    value: &serde_json::Value,
    strings: &mut Vec<super::TranslatableString>,
    prefix: &str,
) {
    match value {
        serde_json::Value::String(s) => {
            if !s.trim().is_empty() {
                strings.push(super::TranslatableString {
                    id: prefix.to_string(),
                    text: s.clone(),
                    _context: Some("TOML string value".to_string()),
                });
            }
        }
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if k.starts_with('_') {
                    continue;
                }
                let new_prefix = format!("{}.{}", prefix, k);
                extract_from_toml(v, strings, &new_prefix);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let new_prefix = format!("{}[{}]", prefix, i);
                extract_from_toml(v, strings, &new_prefix);
            }
        }
        _ => {}
    }
}

fn apply_translations(value: &mut serde_json::Value, translations: &HashMap<String, String>) {
    for (id, translated) in translations {
        let pointer = super::json::id_to_pointer(id);
        if let Some(target) = value.pointer_mut(&pointer) {
            if target.is_string() {
                *target = serde_json::Value::String(translated.to_string());
            }
        } else {
            eprintln!("Warning: path '{}' not found in TOML", id);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::formats::{FormatHandler, TranslatedString};

    #[tokio::test]
    async fn test_toml_extract() {
        let toml_str = r#"
[app]
title = "Welcome"
count = 42

[messages]
greeting = "Hello"
"#;
        let handler = super::TomlHandler::new();
        let strings = handler.extract(toml_str).await.unwrap();
        assert_eq!(strings.len(), 2);
        assert!(strings.iter().any(|s| s.id == "toml.app.title"));
        assert!(strings.iter().any(|s| s.id == "toml.messages.greeting"));
    }

    #[tokio::test]
    async fn test_toml_reconstruct() {
        let toml_str = r#"[app]
title = "Welcome"

[messages]
greeting = "Hello"
"#;
        let handler = super::TomlHandler::new();
        let translations = vec![
            TranslatedString {
                id: "toml.app.title".into(),
                translated_text: "Bienvenue".into(),
            },
            TranslatedString {
                id: "toml.messages.greeting".into(),
                translated_text: "Bonjour".into(),
            },
        ];
        let result = handler.reconstruct(toml_str, &translations).await.unwrap();
        assert!(result.contains("Bienvenue"));
        assert!(result.contains("Bonjour"));
        assert!(result.contains("greeting"));
    }
}
