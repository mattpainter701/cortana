use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryMode {
    Text,
    Code,
    Diff,
}

impl SummaryMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "text" => Ok(Self::Text),
            "code" => Ok(Self::Code),
            "diff" => Ok(Self::Diff),
            other => Err(format!("unsupported summary mode '{other}'")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Code => "code",
            Self::Diff => "diff",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SummaryResult {
    pub mode: SummaryMode,
    pub summary: String,
    pub keywords: Vec<String>,
    pub highlights: Vec<String>,
    pub files: Vec<FileHighlight>,
    pub token_usage: u32,
}

#[derive(Debug, Clone)]
pub struct FileHighlight {
    pub path: String,
    pub kind: String,
    pub highlights: Vec<String>,
}

pub fn summarize(input: &str, mode: SummaryMode, path: Option<&str>) -> SummaryResult {
    match mode {
        SummaryMode::Text => summarize_text(input, path),
        SummaryMode::Code => summarize_code(input, path),
        SummaryMode::Diff => summarize_diff(input),
    }
}

fn summarize_text(input: &str, path: Option<&str>) -> SummaryResult {
    let keywords = top_keywords(input, 6);
    let highlights = top_sentences(input, &keywords, 3);
    let summary = if highlights.is_empty() {
        first_non_empty_line(input)
            .unwrap_or("No summary available.")
            .to_string()
    } else {
        highlights.join(" ")
    };

    SummaryResult {
        mode: SummaryMode::Text,
        summary,
        keywords,
        highlights,
        files: path.map(file_from_path).into_iter().collect(),
        token_usage: 0,
    }
}

fn summarize_code(input: &str, path: Option<&str>) -> SummaryResult {
    let symbols = extract_code_symbols(input);
    let doc_lines = extract_doc_lines(input, 4);
    let keywords = top_keywords(
        &format!("{}\n{}", symbols.join(" "), doc_lines.join(" ")),
        6,
    );
    let mut highlights = Vec::new();

    if !symbols.is_empty() {
        highlights.push(format!(
            "Defines {} notable symbol(s): {}.",
            symbols.len(),
            symbols.join(", ")
        ));
    }
    highlights.extend(
        doc_lines
            .into_iter()
            .map(|line| format!("Doc/comment: {line}")),
    );

    if highlights.is_empty() {
        highlights = top_sentences(input, &keywords, 3);
    }

    let summary = if highlights.is_empty() {
        "Code summary unavailable; no symbols or comments detected.".to_string()
    } else {
        highlights.join(" ")
    };

    let files = path
        .map(|p| FileHighlight {
            path: p.to_string(),
            kind: "code".to_string(),
            highlights: highlights.iter().take(3).cloned().collect(),
        })
        .into_iter()
        .collect();

    SummaryResult {
        mode: SummaryMode::Code,
        summary,
        keywords,
        highlights,
        files,
        token_usage: 0,
    }
}

fn summarize_diff(input: &str) -> SummaryResult {
    let files = parse_diff_files(input);
    let mut highlights = Vec::new();

    for file in &files {
        highlights.push(format!(
            "{} file {} with {} highlight(s).",
            file.kind,
            file.path,
            file.highlights.len()
        ));
    }

    if highlights.is_empty() {
        highlights = top_sentences(input, &top_keywords(input, 6), 3);
    }

    let keywords = top_keywords(&highlights.join(" "), 6);
    let summary = if highlights.is_empty() {
        "Diff summary unavailable; no file changes detected.".to_string()
    } else {
        highlights.join(" ")
    };

    SummaryResult {
        mode: SummaryMode::Diff,
        summary,
        keywords,
        highlights,
        files,
        token_usage: 0,
    }
}

pub fn top_keywords(input: &str, limit: usize) -> Vec<String> {
    let stopwords = stopwords();
    let mut counts: HashMap<String, usize> = HashMap::new();

    for token in tokenize(input) {
        if token.len() < 3 || stopwords.contains(token.as_str()) {
            continue;
        }
        *counts.entry(token).or_default() += 1;
    }

    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_by(|(lw, lc), (rw, rc)| rc.cmp(lc).then_with(|| lw.cmp(rw)));
    ranked
        .into_iter()
        .take(limit)
        .map(|(word, _)| word)
        .collect()
}

fn top_sentences(input: &str, keywords: &[String], limit: usize) -> Vec<String> {
    let keyword_set: HashSet<&str> = keywords.iter().map(String::as_str).collect();
    let sentences = split_sentences(input);
    let mut scored = Vec::new();

    for (index, sentence) in sentences.iter().enumerate() {
        let tokens = tokenize(sentence);
        if tokens.is_empty() {
            continue;
        }
        let keyword_hits = tokens
            .iter()
            .filter(|token| keyword_set.contains(token.as_str()))
            .count();
        let position_bonus = if index == 0 { 2 } else { 0 };
        let score = keyword_hits * 3 + position_bonus + tokens.len().min(20) / 5;
        scored.push((index, score, sentence.trim().to_string()));
    }

    scored.sort_by(|(li, ls, _), (ri, rs, _)| rs.cmp(ls).then_with(|| li.cmp(ri)));
    scored.truncate(limit);
    scored.sort_by_key(|(index, _, _)| *index);
    scored
        .into_iter()
        .map(|(_, _, sentence)| sentence)
        .collect()
}

fn split_sentences(input: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for ch in input.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | '\n') {
            let trimmed = current.trim();
            if trimmed.len() > 20 {
                sentences.push(trimmed.to_string());
            }
            current.clear();
        }
    }

    let trimmed = current.trim();
    if trimmed.len() > 20 {
        sentences.push(trimmed.to_string());
    }

    sentences
}

pub fn tokenize(input: &str) -> Vec<String> {
    input
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn extract_code_symbols(input: &str) -> Vec<String> {
    let mut symbols = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim_start();
        for prefix in [
            "fn ", "pub fn ", "struct ", "pub struct ", "enum ", "pub enum ",
            "trait ", "impl ", "mod ", "pub mod ",
        ] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                let symbol = rest
                    .split(|ch: char| {
                        ch == '(' || ch == '<' || ch == '{' || ch.is_whitespace()
                    })
                    .next()
                    .unwrap_or("")
                    .trim_matches(':');
                if !symbol.is_empty() {
                    symbols.push(symbol.to_string());
                }
            }
        }
    }

    symbols.sort();
    symbols.dedup();
    symbols.truncate(10);
    symbols
}

fn extract_doc_lines(input: &str, limit: usize) -> Vec<String> {
    let mut docs = Vec::new();
    for line in input.lines() {
        let trimmed = line.trim();
        let cleaned = trimmed
            .strip_prefix("///")
            .or_else(|| trimmed.strip_prefix("//!"))
            .or_else(|| trimmed.strip_prefix("//"))
            .or_else(|| {
                if trimmed.starts_with("#[") {
                    None
                } else {
                    trimmed.strip_prefix('#')
                }
            })
            .map(str::trim);

        if let Some(cleaned) = cleaned {
            if cleaned.len() > 10 {
                docs.push(cleaned.to_string());
            }
        }
        if docs.len() >= limit {
            break;
        }
    }
    docs
}

fn parse_diff_files(input: &str) -> Vec<FileHighlight> {
    let mut files: BTreeMap<String, FileHighlight> = BTreeMap::new();
    let mut current_path: Option<String> = None;

    for line in input.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let path = rest
                .split_whitespace()
                .nth(1)
                .unwrap_or("unknown")
                .trim_start_matches("b/")
                .to_string();
            current_path = Some(path.clone());
            files.entry(path.clone()).or_insert(FileHighlight {
                path,
                kind: "modified".to_string(),
                highlights: Vec::new(),
            });
            continue;
        }

        if let Some(path) = line.strip_prefix("+++ b/") {
            current_path = Some(path.to_string());
            files.entry(path.to_string()).or_insert(FileHighlight {
                path: path.to_string(),
                kind: "modified".to_string(),
                highlights: Vec::new(),
            });
            continue;
        }

        let Some(path) = current_path.as_ref() else {
            continue;
        };

        if line.starts_with("new file mode") {
            if let Some(file) = files.get_mut(path) {
                file.kind = "added".to_string();
            }
        } else if line.starts_with("deleted file mode") {
            if let Some(file) = files.get_mut(path) {
                file.kind = "deleted".to_string();
            }
        } else if line.starts_with("@@") {
            if let Some(file) = files.get_mut(path) {
                file.highlights.push(line.to_string());
            }
        } else if let Some(added) = line.strip_prefix('+') {
            if !added.starts_with("+++") && added.trim().len() > 12 {
                if let Some(file) = files.get_mut(path) {
                    file.highlights.push(format!("added: {}", added.trim()));
                }
            }
        }
    }

    for file in files.values_mut() {
        file.highlights.truncate(5);
    }

    files.into_values().collect()
}

pub fn first_non_empty_line(input: &str) -> Option<&str> {
    input.lines().map(str::trim).find(|line| !line.is_empty())
}

fn file_from_path(path: &str) -> FileHighlight {
    let kind = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("text")
        .to_string();
    FileHighlight {
        path: path.to_string(),
        kind,
        highlights: Vec::new(),
    }
}

fn stopwords() -> HashSet<&'static str> {
    [
        "about", "after", "again", "also", "and", "are", "because", "but", "can",
        "for", "from", "has", "have", "into", "not", "only", "our", "should",
        "that", "the", "this", "through", "use", "when", "where", "with",
        "without", "you", "will", "each", "they", "them", "their", "been",
    ]
    .into_iter()
    .collect()
}

impl SummaryResult {
    pub fn to_json(&self) -> String {
        let keywords = json_array(self.keywords.iter().map(String::as_str));
        let highlights = json_array(self.highlights.iter().map(String::as_str));
        let files = self
            .files
            .iter()
            .map(FileHighlight::to_json)
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "{{\"mode\":\"{}\",\"summary\":\"{}\",\"keywords\":{},\"highlights\":{},\"files\":[{}],\"token_usage\":{}}}",
            self.mode.as_str(),
            json_escape(&self.summary),
            keywords,
            highlights,
            files,
            self.token_usage
        )
    }

    pub fn to_banner(&self) -> String {
        let mode_label = match self.mode {
            SummaryMode::Text => "RECAP",
            SummaryMode::Code => "CODE",
            SummaryMode::Diff => "DIFF",
        };
        format!("{mode_label}: {}", self.summary)
    }
}

impl FileHighlight {
    fn to_json(&self) -> String {
        format!(
            "{{\"path\":\"{}\",\"kind\":\"{}\",\"highlights\":{}}}",
            json_escape(&self.path),
            json_escape(&self.kind),
            json_array(self.highlights.iter().map(String::as_str))
        )
    }
}

fn json_array<'a>(items: impl Iterator<Item = &'a str>) -> String {
    let values = items
        .map(|item| format!("\"{}\"", json_escape(item)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

pub fn json_escape(input: &str) -> String {
    let mut escaped = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", ch as u32))
            }
            ch => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_summary_uses_zero_tokens() {
        let input = "Cortana is a terminal assistant. Cortana can summarize documents quickly without LLM tokens. The terminal recap banner stays concise.";
        let result = summarize(input, SummaryMode::Text, None);
        assert_eq!(result.token_usage, 0);
        assert!(result.summary.contains("Cortana"));
        assert!(!result.keywords.is_empty());
    }

    #[test]
    fn code_summary_extracts_symbols() {
        let input = "/// Renders the living face.\npub struct FaceParams {}\npub fn render_face() {}";
        let result = summarize(input, SummaryMode::Code, Some("src/face.rs"));
        assert_eq!(result.token_usage, 0);
        assert!(result.summary.contains("FaceParams"));
        assert!(result.summary.contains("render_face"));
    }

    #[test]
    fn diff_summary_extracts_changed_files() {
        let input = "diff --git a/src/main.rs b/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n+pub fn main() { println!(\"ready\"); }\n";
        let result = summarize(input, SummaryMode::Diff, None);
        assert_eq!(result.token_usage, 0);
        assert_eq!(result.files[0].path, "src/main.rs");
    }
}
