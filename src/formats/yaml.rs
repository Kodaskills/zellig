use async_trait::async_trait;
use std::collections::HashMap;

pub struct YamlHandler;

impl YamlHandler {
    pub fn new() -> Self {
        YamlHandler
    }
}

impl Default for YamlHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::FormatHandler for YamlHandler {
    async fn extract(&self, content: &str) -> crate::error::Result<Vec<super::TranslatableString>> {
        let value: serde_json::Value = serde_yaml::from_str(content)
            .map_err(|e| crate::error::ZelligError::ConfigError(e.to_string()))?;
        let mut strings = Vec::new();
        extract_from_yaml(&value, &mut strings, "yaml");
        Ok(strings)
    }

    async fn reconstruct(
        &self,
        original: &str,
        translations: &[super::TranslatedString],
    ) -> crate::error::Result<String> {
        let mut value: serde_json::Value = serde_yaml::from_str(original)
            .map_err(|e| crate::error::ZelligError::ConfigError(e.to_string()))?;
        let translation_map: HashMap<String, String> = translations
            .iter()
            .map(|t| (t.id.clone(), t.translated_text.clone()))
            .collect();
        apply_translations(&mut value, &translation_map);
        serde_yaml::to_string(&value)
            .map_err(|e| crate::error::ZelligError::ConfigError(e.to_string()))
    }
}

fn extract_from_yaml(
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
                    _context: Some("YAML string value".to_string()),
                });
            }
        }
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if k.starts_with('_') {
                    continue;
                }
                let new_prefix = format!("{}.{}", prefix, k);
                extract_from_yaml(v, strings, &new_prefix);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let new_prefix = format!("{}[{}]", prefix, i);
                extract_from_yaml(v, strings, &new_prefix);
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
            eprintln!("Warning: path '{}' not found in YAML", id);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::formats::{FormatHandler, TranslatedString};

    #[tokio::test]
    async fn test_yaml_extract() {
        let yaml_str = r#"
app:
  title: "Welcome"
  count: 42

messages:
  greeting: "Hello"
"#;
        let handler = super::YamlHandler::new();
        let strings = handler.extract(yaml_str).await.unwrap();
        assert_eq!(strings.len(), 2);
        assert!(strings.iter().any(|s| s.id == "yaml.app.title"));
        assert!(strings.iter().any(|s| s.id == "yaml.messages.greeting"));
    }

    #[tokio::test]
    async fn test_yaml_reconstruct() {
        let yaml_str = r#"
app:
  title: "Welcome"
messages:
  greeting: "Hello"
"#;
        let handler = super::YamlHandler::new();
        let translations = vec![
            TranslatedString {
                id: "yaml.app.title".into(),
                translated_text: "Bienvenue".into(),
            },
            TranslatedString {
                id: "yaml.messages.greeting".into(),
                translated_text: "Bonjour".into(),
            },
        ];
        let result = handler.reconstruct(yaml_str, &translations).await.unwrap();
        assert!(result.contains("Bienvenue"));
        assert!(result.contains("Bonjour"));
    }
}
