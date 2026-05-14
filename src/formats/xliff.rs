use async_trait::async_trait;
use quick_xml::events::Event;
use quick_xml::Reader;
use quick_xml::XmlVersion;
use std::collections::HashMap;

pub struct XliffHandler;

impl XliffHandler {
    pub fn new() -> Self {
        XliffHandler
    }
}

impl Default for XliffHandler {
    fn default() -> Self {
        Self::new()
    }
}

fn is_tag(e: &quick_xml::events::BytesStart, name: &[u8]) -> bool {
    let q = e.name();
    let raw = q.as_ref();
    tag_local(raw) == name
}

fn is_end(e: &quick_xml::events::BytesEnd, name: &[u8]) -> bool {
    tag_local(e.name().as_ref()) == name
}

fn tag_local(name: &[u8]) -> &[u8] {
    if let Some(pos) = name.iter().rposition(|&b| b == b'}') {
        &name[pos + 1..]
    } else {
        name
    }
}

fn attr_value(e: &quick_xml::events::BytesStart, name: &[u8]) -> Option<String> {
    for a in e.attributes().flatten() {
        if a.key.as_ref() == name {
            return Some(
                a.normalized_value(XmlVersion::Implicit1_0)
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            );
        }
    }
    None
}

#[async_trait]
impl super::FormatHandler for XliffHandler {
    async fn extract(&self, content: &str) -> crate::error::Result<Vec<super::TranslatableString>> {
        let mut reader = Reader::from_str(content);
        let mut buf = Vec::new();
        let mut strings = Vec::new();
        let mut in_source = false;
        let mut current_id = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if is_tag(e, b"trans-unit") {
                        current_id = attr_value(e, b"id").unwrap_or_default();
                    } else if is_tag(e, b"source") {
                        in_source = true;
                    }
                }
                Ok(Event::Text(ref e)) if in_source => {
                    let text = String::from_utf8_lossy(e.as_ref()).into_owned();
                    let t = text.trim().to_string();
                    if !t.is_empty() {
                        strings.push(super::TranslatableString {
                            id: format!("xliff.{}", current_id),
                            text: t,
                            _context: Some("XLIFF source text".to_string()),
                        });
                    }
                }
                Ok(Event::End(ref e)) if is_end(e, b"source") => {
                    in_source = false;
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(crate::error::ZelligError::ConfigError(format!(
                        "XLIFF parse error: {}",
                        e
                    )));
                }
                _ => {}
            }
            buf.clear();
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
            .map(|t| {
                let id = t.id.strip_prefix("xliff.").unwrap_or(&t.id).to_string();
                (id, t.translated_text.clone())
            })
            .collect();

        let mut reader = Reader::from_str(original);
        let mut buf = Vec::new();
        let mut edits: Vec<(usize, usize, String)> = Vec::new();

        let mut in_trans_unit = false;
        let mut in_target = false;
        let mut current_id = String::new();
        let mut target_start = 0usize;

        loop {
            let before = reader.buffer_position() as usize;
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    if is_tag(e, b"trans-unit") {
                        in_trans_unit = true;
                        current_id = attr_value(e, b"id").unwrap_or_default();
                    } else if is_tag(e, b"target") && in_trans_unit {
                        in_target = true;
                        target_start = reader.buffer_position() as usize;
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    if is_tag(e, b"target") && in_trans_unit {
                        if let Some(translated) = translation_map.get(&current_id) {
                            let after = reader.buffer_position() as usize;
                            edits.push((before, after, format!("<target>{}</target>", translated)));
                        }
                    } else if is_tag(e, b"target") {
                        // target outside trans-unit, ignore but still need in_target = false
                        in_target = false;
                    }
                }
                Ok(Event::Text(_)) if in_target => {
                    if let Some(translated) = translation_map.get(&current_id) {
                        let text_end = reader.buffer_position() as usize;
                        edits.push((target_start, text_end, translated.clone()));
                    }
                }
                Ok(Event::End(ref e)) => {
                    if is_end(e, b"target") {
                        in_target = false;
                    } else if is_end(e, b"trans-unit") {
                        in_trans_unit = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(crate::error::ZelligError::ConfigError(format!(
                        "XLIFF parse error: {}",
                        e
                    )));
                }
                _ => {}
            }
            buf.clear();
        }

        let mut result = original.to_string();
        for (start, end, replacement) in edits.into_iter().rev() {
            result.replace_range(start..end, &replacement);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::formats::{FormatHandler, TranslatedString};

    #[tokio::test]
    async fn test_xliff_extract() {
        let xliff = r#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file source-language="en" target-language="fr" datatype="plaintext">
    <body>
      <trans-unit id="1">
        <source>Welcome</source>
        <target/>
      </trans-unit>
    </body>
  </file>
</xliff>"#;
        let handler = super::XliffHandler::new();
        let strings = handler.extract(xliff).await.unwrap();
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].id, "xliff.1");
        assert_eq!(strings[0].text, "Welcome");
    }

    #[tokio::test]
    async fn test_xliff_reconstruct_self_closing() {
        let xliff = r#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file source-language="en" target-language="fr" datatype="plaintext">
    <body>
      <trans-unit id="1">
        <source>Welcome</source>
        <target/>
      </trans-unit>
    </body>
  </file>
</xliff>"#;
        let handler = super::XliffHandler::new();
        let translations = vec![TranslatedString {
            id: "xliff.1".into(),
            translated_text: "Bonjour".into(),
        }];
        let result = handler.reconstruct(xliff, &translations).await.unwrap();
        assert!(result.contains("<target>Bonjour</target>"));
    }

    #[tokio::test]
    async fn test_xliff_reconstruct_existing_target() {
        let xliff = r#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file source-language="en" target-language="fr" datatype="plaintext">
    <body>
      <trans-unit id="1">
        <source>Welcome</source>
        <target>Old</target>
      </trans-unit>
    </body>
  </file>
</xliff>"#;
        let handler = super::XliffHandler::new();
        let translations = vec![TranslatedString {
            id: "xliff.1".into(),
            translated_text: "Bonjour".into(),
        }];
        let result = handler.reconstruct(xliff, &translations).await.unwrap();
        assert!(result.contains("<target>Bonjour</target>"));
        assert!(!result.contains("<target>Old</target>"));
    }
}
