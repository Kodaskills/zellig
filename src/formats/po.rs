use async_trait::async_trait;
use std::collections::HashMap;

pub struct PoHandler;

impl PoHandler {
    pub fn new() -> Self {
        PoHandler
    }
}

impl Default for PoHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::FormatHandler for PoHandler {
    async fn extract(&self, content: &str) -> crate::error::Result<Vec<super::TranslatableString>> {
        let items = parse_msgstr_lines(content);
        let mut strings = Vec::new();
        let mut idx = 0usize;
        for item in &items {
            if !item.text.is_empty() {
                strings.push(super::TranslatableString {
                    id: format!("po.{}", idx),
                    text: item.text.clone(),
                    _context: Some("PO msgstr text".to_string()),
                });
                idx += 1;
            }
        }
        Ok(strings)
    }

    async fn reconstruct(
        &self,
        original: &str,
        translations: &[super::TranslatedString],
    ) -> crate::error::Result<String> {
        let translation_map: HashMap<String, String> = translations
            .iter()
            .map(|t| (t.id.clone(), t.translated_text.clone()))
            .collect();
        let items = parse_msgstr_lines(original);
        let mut replacements: Vec<(usize, usize, String)> = Vec::new();
        let mut idx = 0usize;
        for item in &items {
            if item.text.is_empty() {
                continue;
            }
            let key = format!("po.{}", idx);
            if let Some(translated) = translation_map.get(&key) {
                replacements.push((item.start, item.end, translated.clone()));
            }
            idx += 1;
        }
        replacements.sort_by_key(|b| std::cmp::Reverse(b.1));
        let mut result = original.to_string();
        for (start, end, text) in &replacements {
            result.replace_range(*start..*end, text);
        }
        Ok(result)
    }
}

struct MsgstrLine {
    text: String,
    start: usize,
    end: usize,
}

fn parse_msgstr_lines(content: &str) -> Vec<MsgstrLine> {
    let mut items = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("msgstr") {
            let (start, end) = msgstr_inner_byte_range(content, line, trimmed);
            let text = extract_msgstr_value(trimmed).unwrap_or_default();
            items.push(MsgstrLine { text, start, end });
        }
    }
    items
}

fn extract_msgstr_value(s: &str) -> Option<String> {
    let s = s.trim();
    let quote_start = s.find('"')?;
    let s_after = &s[quote_start..];
    let val = s_after.strip_prefix('"')?.strip_suffix('"')?;
    Some(val.to_string())
}

fn msgstr_inner_byte_range(content: &str, line: &str, trimmed: &str) -> (usize, usize) {
    let line_start = line.as_ptr() as usize - content.as_ptr() as usize;
    if let Some(quote_start) = trimmed.find('"') {
        let inner_start = line_start + trimmed[..quote_start + 1].len();
        let inner_end = line_start + trimmed.len() - 1;
        (inner_start, inner_end)
    } else {
        (0, 0)
    }
}

#[cfg(test)]
mod tests {
    use crate::formats::{FormatHandler, TranslatedString};

    #[tokio::test]
    async fn test_po_extract_from_msgstr() {
        let po = r#"msgid ""
msgstr ""

msgid "app.title"
msgstr "Welcome to Zellig"

msgid "messages.greeting"
msgid_plural "messages"
msgstr[0] "Hello"
msgstr[1] "Hellos"
"#;
        let handler = super::PoHandler::new();
        let strings = handler.extract(po).await.unwrap();
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[0].text, "Welcome to Zellig");
        assert_eq!(strings[0].id, "po.0");
        assert_eq!(strings[1].text, "Hello");
        assert_eq!(strings[1].id, "po.1");
        assert_eq!(strings[2].text, "Hellos");
        assert_eq!(strings[2].id, "po.2");
    }

    #[tokio::test]
    async fn test_po_reconstruct_plural() {
        let po = r#"msgid "app.title"
msgstr "Welcome to Zellig"

msgid "messages.greeting"
msgid_plural "messages"
msgstr[0] "Hello"
msgstr[1] "Hellos"
"#;
        let handler = super::PoHandler::new();
        let translations = vec![
            TranslatedString {
                id: "po.0".into(),
                translated_text: "Bienvenue sur Zellig".into(),
            },
            TranslatedString {
                id: "po.1".into(),
                translated_text: "Bonjour".into(),
            },
            TranslatedString {
                id: "po.2".into(),
                translated_text: "Bonjours".into(),
            },
        ];
        let result = handler.reconstruct(po, &translations).await.unwrap();
        assert!(result.contains(r#"msgstr "Bienvenue sur Zellig""#));
        assert!(result.contains(r#"msgstr[0] "Bonjour""#));
        assert!(result.contains(r#"msgstr[1] "Bonjours""#));
        assert!(result.contains(r#"msgid "app.title""#));
        assert!(result.contains(r#"msgid_plural "messages""#));
    }

    #[tokio::test]
    async fn test_po_skips_empty_header() {
        let po = r#"msgid ""
msgstr ""

msgid "hello"
msgstr "Hello"
"#;
        let handler = super::PoHandler::new();
        let strings = handler.extract(po).await.unwrap();
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].text, "Hello");
        assert_eq!(strings[0].id, "po.0");
    }
}
