mod face;
mod visual;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SummaryMode {
    Text,
    Code,
    Diff,
}

impl SummaryMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "text" => Ok(Self::Text),
            "code" => Ok(Self::Code),
            "diff" => Ok(Self::Diff),
            other => Err(format!("unsupported summary mode '{other}'")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Code => "code",
            Self::Diff => "diff",
        }
    }
}

#[derive(Debug, Clone)]
struct SummaryResult {
    mode: SummaryMode,
    summary: String,
    keywords: Vec<String>,
    highlights: Vec<String>,
    files: Vec<FileHighlight>,
    token_usage: u32,
}

#[derive(Debug, Clone)]
struct FileHighlight {
    path: String,
    kind: String,
    highlights: Vec<String>,
}

#[derive(Debug, Clone)]
struct SummaryOptions {
    fast: bool,
    mode: SummaryMode,
    json: bool,
    input_path: Option<String>,
}

#[derive(Debug, Clone)]
enum Command {
    Help,
    Version,
    Boot,
    Appearance,
    Summarize(SummaryOptions),
    Recap {
        session: String,
        format: String,
        speak: bool,
    },
    Speak(String),
    Daemon {
        stdio: bool,
    },
    SessionStart,
}

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("error: {error}");
        eprintln!("\nRun `cortana --help` for usage.");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match parse_args(&args)? {
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Version => {
            println!("cortana {VERSION}");
            Ok(())
        }
        Command::Boot => {
            print!("{}", visual::boot_splash());
            Ok(())
        }
        Command::Appearance => {
            print!("{}", visual::appearance_splash());
            Ok(())
        }
        Command::Summarize(options) => {
            if !options.fast {
                return Err(
                    "only --fast no-token summaries are implemented in this prototype".into(),
                );
            }
            let input = read_input(options.input_path.as_deref())?;
            let result = summarize(&input, options.mode, options.input_path.as_deref());
            if options.json {
                println!("{}", result.to_json());
            } else {
                print_text_summary(&result);
            }
            Ok(())
        }
        Command::Recap {
            session,
            format,
            speak,
        } => {
            if session != "current" {
                return Err("only `--session current` is implemented in this prototype".into());
            }
            if format != "markdown" {
                return Err("only `--format markdown` is implemented in this prototype".into());
            }
            println!("# Cortana Session Recap\n");
            println!("- Session: current");
            println!("- Summary mode: fast/no-token");
            println!("- Status: recap command scaffold is wired and ready for session storage integration.");
            if speak {
                println!(
                    "\nSPEAK: Session recap is ready. Session storage integration is pending."
                );
            }
            Ok(())
        }
        Command::Speak(message) => {
            println!("SPEAK: {message}");
            Ok(())
        }
        Command::Daemon { stdio } => {
            if !stdio {
                return Err("daemon currently requires --stdio".into());
            }
            let mut face = face::FaceState::default();
            let target = face::FaceParams::from_signals(0.15, 0.4, 0.2, 0.0);
            face.tick_toward(target, 0.016);
            print!("{}", visual::session_start_banner());
            println!("cortana daemon ready on stdio");
            println!("face_params: {}", face.params().to_protocol_line());
            Ok(())
        }
        Command::SessionStart => {
            print!("{}", visual::session_start_banner());
            Ok(())
        }
    }
}

fn parse_args(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Ok(Command::Boot);
    }

    match args[0].as_str() {
        "--help" | "-h" | "help" => Ok(Command::Help),
        "--version" | "-V" | "version" => Ok(Command::Version),
        "boot" => Ok(Command::Boot),
        "appear" | "appearance" => Ok(Command::Appearance),
        "summarize" => parse_summarize(&args[1..]),
        "recap" => parse_recap(&args[1..]),
        "speak" => parse_speak(&args[1..]),
        "daemon" => parse_daemon(&args[1..]),
        "session" => parse_session(&args[1..]),
        other => Err(format!("unknown command '{other}'")),
    }
}

fn parse_session(args: &[String]) -> Result<Command, String> {
    match args {
        [command] if command == "start" => Ok(Command::SessionStart),
        [] => Err("session requires a subcommand; supported: start".into()),
        [other, ..] => Err(format!("unknown session subcommand '{other}'")),
    }
}

fn parse_summarize(args: &[String]) -> Result<Command, String> {
    let mut fast = false;
    let mut mode = SummaryMode::Text;
    let mut json = false;
    let mut input_path = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--fast" => fast = true,
            "--json" => json = true,
            "--mode" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--mode requires a value".to_string())?;
                mode = SummaryMode::parse(value)?;
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown summarize flag '{flag}'"))
            }
            path => {
                if input_path.is_some() {
                    return Err(
                        "summarize accepts at most one input path; use stdin for combined input"
                            .into(),
                    );
                }
                input_path = Some(path.to_string());
            }
        }
        index += 1;
    }

    Ok(Command::Summarize(SummaryOptions {
        fast,
        mode,
        json,
        input_path,
    }))
}

fn parse_recap(args: &[String]) -> Result<Command, String> {
    let mut session = "current".to_string();
    let mut format = "markdown".to_string();
    let mut speak = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--session" => {
                index += 1;
                session = args
                    .get(index)
                    .ok_or_else(|| "--session requires a value".to_string())?
                    .clone();
            }
            "--format" => {
                index += 1;
                format = args
                    .get(index)
                    .ok_or_else(|| "--format requires a value".to_string())?
                    .clone();
            }
            "--speak" => speak = true,
            flag => return Err(format!("unknown recap flag '{flag}'")),
        }
        index += 1;
    }

    Ok(Command::Recap {
        session,
        format,
        speak,
    })
}

fn parse_speak(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Err("speak requires a message".into());
    }
    Ok(Command::Speak(args.join(" ")))
}

fn parse_daemon(args: &[String]) -> Result<Command, String> {
    let mut stdio = false;
    for arg in args {
        match arg.as_str() {
            "--stdio" => stdio = true,
            flag => return Err(format!("unknown daemon flag '{flag}'")),
        }
    }
    Ok(Command::Daemon { stdio })
}

fn print_help() {
    println!(
        "Cortana {VERSION}\n\n\
Terminal-first AI assistant prototype.\n\n\
USAGE:\n\
  cortana boot\n\
  cortana appear\n\
  cortana session start\n\
  cortana summarize --fast [--mode text|code|diff] [--json] [path]\n\
  cortana recap --session current --format markdown [--speak]\n\
  cortana speak <message>\n\
  cortana daemon --stdio\n\n\
EXAMPLES:\n\
  cortana boot\n\
  cortana session start\n\
  cortana summarize --fast --mode text CORTANA_IMPLEMENTATION.md\n\
  git diff | cortana summarize --fast --mode diff --json\n\
  cortana speak \"Build completed successfully.\""
    );
}

fn read_input(path: Option<&str>) -> Result<String, String> {
    if let Some(path) = path {
        return fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"));
    }

    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|error| format!("failed to read stdin: {error}"))?;

    if buffer.trim().is_empty() {
        return Err("no input provided; pass a path or pipe text on stdin".into());
    }

    Ok(buffer)
}

fn summarize(input: &str, mode: SummaryMode, path: Option<&str>) -> SummaryResult {
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
        .map(|path| FileHighlight {
            path: path.to_string(),
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

fn top_keywords(input: &str, limit: usize) -> Vec<String> {
    let stopwords = stopwords();
    let mut counts: HashMap<String, usize> = HashMap::new();

    for token in tokenize(input) {
        if token.len() < 3 || stopwords.contains(token.as_str()) {
            continue;
        }
        *counts.entry(token).or_default() += 1;
    }

    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_by(|(left_word, left_count), (right_word, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_word.cmp(right_word))
    });
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

    scored.sort_by(
        |(left_index, left_score, _), (right_index, right_score, _)| {
            right_score
                .cmp(left_score)
                .then_with(|| left_index.cmp(right_index))
        },
    );
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

    for character in input.chars() {
        current.push(character);
        if matches!(character, '.' | '!' | '?' | '\n') {
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

fn tokenize(input: &str) -> Vec<String> {
    input
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn extract_code_symbols(input: &str) -> Vec<String> {
    let mut symbols = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim_start();
        for prefix in [
            "fn ",
            "pub fn ",
            "struct ",
            "pub struct ",
            "enum ",
            "pub enum ",
            "trait ",
            "impl ",
        ] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                let symbol = rest
                    .split(|character: char| {
                        character == '('
                            || character == '<'
                            || character == '{'
                            || character.is_whitespace()
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

fn first_non_empty_line(input: &str) -> Option<&str> {
    input.lines().map(str::trim).find(|line| !line.is_empty())
}

fn file_from_path(path: &str) -> FileHighlight {
    let kind = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("text")
        .to_string();
    FileHighlight {
        path: path.to_string(),
        kind,
        highlights: Vec::new(),
    }
}

fn print_text_summary(result: &SummaryResult) {
    println!("RECAP: {}", result.summary);
    if !result.keywords.is_empty() {
        println!("KEYWORDS: {}", result.keywords.join(", "));
    }
    println!("TOKEN_USAGE: {}", result.token_usage);
}

impl SummaryResult {
    fn to_json(&self) -> String {
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

fn json_escape(input: &str) -> String {
    let mut escaped = String::new();
    for character in input.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32))
            }
            character => escaped.push(character),
        }
    }
    escaped
}

fn stopwords() -> HashSet<&'static str> {
    [
        "about", "after", "again", "also", "and", "are", "because", "but", "can", "for", "from",
        "has", "have", "into", "not", "only", "our", "should", "that", "the", "this", "through",
        "use", "when", "where", "with", "without", "you",
    ]
    .into_iter()
    .collect()
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
        let input = r#"
            /// Renders the living face.
            pub struct FaceParams {}
            pub fn render_face() {}
        "#;
        let result = summarize(input, SummaryMode::Code, Some("src/face.rs"));

        assert_eq!(result.token_usage, 0);
        assert!(result.summary.contains("FaceParams"));
        assert!(result.summary.contains("render_face"));
        assert_eq!(result.files[0].path, "src/face.rs");
    }

    #[test]
    fn diff_summary_extracts_changed_files() {
        let input = "diff --git a/src/main.rs b/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n+pub fn main() { println!(\"ready\"); }\n";
        let result = summarize(input, SummaryMode::Diff, None);

        assert_eq!(result.token_usage, 0);
        assert_eq!(result.files[0].path, "src/main.rs");
        assert!(result.summary.contains("src/main.rs"));
    }

    #[test]
    fn json_escapes_strings() {
        let result = SummaryResult {
            mode: SummaryMode::Text,
            summary: "quoted \"text\"".to_string(),
            keywords: vec!["terminal".to_string()],
            highlights: vec![],
            files: vec![],
            token_usage: 0,
        };

        assert!(result.to_json().contains("quoted \\\"text\\\""));
    }
}
