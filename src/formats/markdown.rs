use async_trait::async_trait;
use std::collections::HashMap;

pub struct MarkdownHandler;

impl MarkdownHandler {
    pub fn new() -> Self {
        MarkdownHandler
    }
}

impl Default for MarkdownHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::FormatHandler for MarkdownHandler {
    async fn extract(&self, content: &str) -> crate::error::Result<Vec<super::TranslatableString>> {
        let segments = extract_md_segments(content);
        let mut strings = Vec::new();
        for seg in &segments {
            strings.push(super::TranslatableString {
                id: seg.id.clone(),
                text: seg.text.clone(),
                _context: Some(format!("Markdown {:?}", seg.kind)),
            });
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
        let mut segments = extract_md_segments(original);
        segments.sort_by_key(|b| std::cmp::Reverse(b.end));
        let mut result = original.to_string();
        for seg in &segments {
            if let Some(translated) = translation_map.get(&seg.id) {
                result.replace_range(seg.start..seg.end, translated);
            }
        }
        Ok(result)
    }
}

#[derive(Debug)]
enum SegmentKind {
    Heading,
    Paragraph,
    ListItem,
    Blockquote,
}

#[derive(Debug)]
struct Segment {
    id: String,
    text: String,
    kind: SegmentKind,
    start: usize,
    end: usize,
}

fn extract_md_segments(content: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut heading_idx = 0usize;
    let mut para_idx = 0usize;
    let mut li_idx = 0usize;
    let mut bq_idx = 0usize;

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        if trimmed.starts_with("```") {
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                i += 1;
            }
            i += 1;
            continue;
        }

        if trimmed.starts_with("> ") || trimmed.starts_with(">") {
            let bq_content = trimmed.trim_start_matches('>').trim();
            let (start, end) = content_range_in_line(content, i, trimmed, bq_content);
            let id = format!("md.blockquote.{}", bq_idx);
            bq_idx += 1;
            segments.push(Segment {
                id,
                text: bq_content.to_string(),
                kind: SegmentKind::Blockquote,
                start,
                end,
            });
            i += 1;
            continue;
        }

        if trimmed.starts_with('-') || trimmed.starts_with('*') {
            let li_content = trimmed.trim_start_matches(&['-', '*', ' '][..]).trim();
            let (start, end) = content_range_in_line(content, i, trimmed, li_content);
            let id = format!("md.li.{}", li_idx);
            li_idx += 1;
            segments.push(Segment {
                id,
                text: li_content.to_string(),
                kind: SegmentKind::ListItem,
                start,
                end,
            });
            i += 1;
            continue;
        }

        if trimmed.starts_with('#') {
            let heading = trimmed.trim_start_matches('#').trim();
            let (start, end) = content_range_in_line(content, i, trimmed, heading);
            let id = format!("md.h{}.{}", count_leading(trimmed, '#'), heading_idx);
            heading_idx += 1;
            segments.push(Segment {
                id,
                text: heading.to_string(),
                kind: SegmentKind::Heading,
                start,
                end,
            });
            i += 1;
            continue;
        }

        let start = line_start_byte(content, i);
        let mut para_lines = vec![line];
        i += 1;
        while i < lines.len() {
            let next = lines[i].trim();
            if next.is_empty()
                || next.starts_with('#')
                || next.starts_with('>')
                || next.starts_with('-')
                || next.starts_with('*')
                || next.starts_with("```")
            {
                break;
            }
            para_lines.push(lines[i]);
            i += 1;
        }
        let para_text = para_lines.join("\n");
        let end = line_end_byte(content, i - 1);
        if !para_text.trim().is_empty() {
            let id = format!("md.p.{}", para_idx);
            para_idx += 1;
            segments.push(Segment {
                id,
                text: para_text.trim().to_string(),
                kind: SegmentKind::Paragraph,
                start,
                end,
            });
        }
    }

    segments
}

fn content_range_in_line(
    content: &str,
    line_idx: usize,
    line: &str,
    content_text: &str,
) -> (usize, usize) {
    let line_start = line_start_byte(content, line_idx);
    let idx = line.find(content_text).unwrap_or(0);
    (line_start + idx, line_start + idx + content_text.len())
}

fn line_start_byte(content: &str, line_idx: usize) -> usize {
    let mut pos = 0;
    for (i, line) in content.lines().enumerate() {
        if i == line_idx {
            return pos;
        }
        pos += line.len() + 1;
    }
    content.len()
}

fn line_end_byte(content: &str, line_idx: usize) -> usize {
    let mut pos = 0;
    for (i, line) in content.lines().enumerate() {
        pos += line.len();
        if i == line_idx {
            return pos;
        }
        pos += 1;
    }
    content.len()
}

fn count_leading(s: &str, c: char) -> usize {
    s.chars().take_while(|&ch| ch == c).count()
}

#[cfg(test)]
mod tests {
    use crate::formats::{FormatHandler, TranslatedString};

    #[tokio::test]
    async fn test_markdown_extract() {
        let md = r#"# Welcome

Hello world.

- item one
- item two

> A quote
"#;
        let handler = super::MarkdownHandler::new();
        let strings = handler.extract(md).await.unwrap();
        assert!(strings.iter().any(|s| s.id.starts_with("md.h1.")));
        assert!(strings.iter().any(|s| s.id.starts_with("md.p.")));
        assert!(strings.iter().any(|s| s.id.starts_with("md.li.")));
        assert!(strings.iter().any(|s| s.id.starts_with("md.blockquote.")));
    }

    #[tokio::test]
    async fn test_markdown_reconstruct() {
        let md = "# Welcome\n\nHello world.\n";
        let handler = super::MarkdownHandler::new();
        let translations = vec![
            TranslatedString {
                id: "md.h1.0".into(),
                translated_text: "Bienvenue".into(),
            },
            TranslatedString {
                id: "md.p.0".into(),
                translated_text: "Bonjour le monde.".into(),
            },
        ];
        let result = handler.reconstruct(md, &translations).await.unwrap();
        assert_eq!(result, "# Bienvenue\n\nBonjour le monde.\n");
    }
}
