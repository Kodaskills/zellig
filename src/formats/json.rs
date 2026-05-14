use async_trait::async_trait;
use sonic_rs::{JsonContainerTrait, JsonValueMutTrait, JsonValueTrait};
use std::collections::HashMap;

pub struct JsonHandler;

impl JsonHandler {
    pub fn new() -> Self {
        JsonHandler
    }
}

impl Default for JsonHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::FormatHandler for JsonHandler {
    async fn extract(&self, content: &str) -> crate::error::Result<Vec<super::TranslatableString>> {
        let mut strings = Vec::new();
        let value: sonic_rs::Value = sonic_rs::from_str(content)
            .map_err(|e| crate::error::ZelligError::ConfigError(e.to_string()))?;
        extract_from_json(&value, &mut strings, "json");
        Ok(strings)
    }

    async fn reconstruct(
        &self,
        original: &str,
        translations: &[super::TranslatedString],
    ) -> crate::error::Result<String> {
        let mut value: sonic_rs::Value = sonic_rs::from_str(original)
            .map_err(|e| crate::error::ZelligError::ConfigError(e.to_string()))?;
        let translation_map: HashMap<String, String> = translations
            .iter()
            .map(|t| (t.id.clone(), t.translated_text.clone()))
            .collect();
        apply_translations_json(&mut value, &translation_map);
        sonic_rs::to_string_pretty(&value)
            .map_err(|e| crate::error::ZelligError::ConfigError(e.to_string()))
    }
}

pub(crate) fn extract_from_json(
    value: &sonic_rs::Value,
    strings: &mut Vec<super::TranslatableString>,
    prefix: &str,
) {
    if let Some(s) = value.as_str() {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            strings.push(super::TranslatableString {
                id: prefix.to_string(),
                text: trimmed.to_string(),
                _context: Some("JSON string value".to_string()),
            });
        }
    } else if let Some(map) = value.as_object() {
        for (k, v) in map {
            if k.starts_with('_') {
                continue;
            }
            let new_prefix = format!("{}.{}", prefix, k);
            extract_from_json(v, strings, &new_prefix);
        }
    } else if let Some(arr) = value.as_array() {
        for (i, v) in arr.iter().enumerate() {
            let new_prefix = format!("{}[{}]", prefix, i);
            extract_from_json(v, strings, &new_prefix);
        }
    }
}

fn pointer_segments(pointer: &str) -> Vec<String> {
    pointer
        .split('/')
        .skip(1)
        .map(|s| s.replace("~1", "/").replace("~0", "~"))
        .collect()
}

fn get_mut_by_segments<'a>(
    value: &'a mut sonic_rs::Value,
    segments: &[String],
) -> Option<&'a mut sonic_rs::Value> {
    let mut current: *mut sonic_rs::Value = value;
    for seg in segments {
        current = unsafe {
            let val = &mut *current;
            if val.is_object() {
                let map = val.as_object_mut()?;
                map.get_mut(seg)? as *mut sonic_rs::Value
            } else if val.is_array() {
                let idx: usize = seg.parse().ok()?;
                let arr = val.as_array_mut()?;
                arr.get_mut(idx)? as *mut sonic_rs::Value
            } else {
                return None;
            }
        };
    }
    unsafe { Some(&mut *current) }
}

fn apply_translations_json(value: &mut sonic_rs::Value, translations: &HashMap<String, String>) {
    for (id, translated) in translations {
        let pointer = super::json::id_to_pointer(id);
        let segments: Vec<String> = pointer_segments(&pointer);
        if let Some(target) = get_mut_by_segments(value, &segments) {
            if target.is_str() {
                *target = sonic_rs::Value::from(translated.as_str());
            }
        } else {
            eprintln!("Warning: path '{}' not found in JSON", id);
        }
    }
}

pub(crate) fn id_to_pointer(id: &str) -> String {
    let inner = id.split_once('.').map(|(_, rest)| rest).unwrap_or(id);
    let mut pointer = String::new();
    let mut key = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '.' => {
                if !key.is_empty() {
                    pointer.push('/');
                    pointer.push_str(&escape_pointer(&key));
                    key.clear();
                }
            }
            '[' => {
                if !key.is_empty() {
                    pointer.push('/');
                    pointer.push_str(&escape_pointer(&key));
                    key.clear();
                }
                let mut idx = String::new();
                for c in &mut chars {
                    if c == ']' {
                        break;
                    }
                    idx.push(c);
                }
                pointer.push('/');
                pointer.push_str(&idx);
            }
            _ => key.push(c),
        }
    }
    if !key.is_empty() {
        pointer.push('/');
        pointer.push_str(&escape_pointer(&key));
    }
    pointer
}

fn escape_pointer(s: &str) -> String {
    s.replace('~', "~0").replace('/', "~1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_to_pointer() {
        assert_eq!(id_to_pointer("json.app.title"), "/app/title");
        assert_eq!(
            id_to_pointer("json.messages.greeting"),
            "/messages/greeting"
        );
        assert_eq!(id_to_pointer("json.status[0]"), "/status/0");
        assert_eq!(id_to_pointer("json.items[2].name"), "/items/2/name");
    }

    #[test]
    fn test_apply_translations() {
        let mut value = sonic_rs::json!({
            "app": {
                "title": "Welcome",
                "subtitle": "Your tool",
                "count": 42
            },
            "messages": {
                "greeting": "Hello"
            },
            "items": ["a", "b", "c"]
        });

        let mut translations = HashMap::new();
        translations.insert("json.app.title".into(), "Bienvenue".into());
        translations.insert("json.messages.greeting".into(), "Bonjour".into());
        translations.insert("json.items[2]".into(), "C".into());

        apply_translations_json(&mut value, &translations);

        assert_eq!(value["app"]["title"], "Bienvenue");
        assert_eq!(value["app"]["subtitle"], "Your tool");
        assert_eq!(value["app"]["count"], 42);
        assert_eq!(value["messages"]["greeting"], "Bonjour");
        assert_eq!(value["items"][2], "C");
    }
}
