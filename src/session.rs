use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;

/// A single message in a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// A named session with conversation history and working context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub messages: VecDeque<SessionMessage>,
    /// What the user was working on (a short context line).
    pub context: String,
    /// Files the user touched during this session.
    pub files_touched: Vec<String>,
    /// The last recap banner shown.
    pub last_recap: String,
}

impl Session {
    pub fn new(name: &str) -> Self {
        let now = chrono_now();
        Self {
            name: name.to_string(),
            created_at: now.clone(),
            updated_at: now,
            messages: VecDeque::with_capacity(200),
            context: String::new(),
            files_touched: Vec::new(),
            last_recap: String::new(),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        if self.messages.len() >= 200 {
            self.messages.pop_front();
        }
        self.messages.push_back(SessionMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: chrono_now(),
        });
        self.updated_at = chrono_now();
    }

    pub fn set_context(&mut self, context: &str) {
        self.context = context.to_string();
        self.updated_at = chrono_now();
    }

    pub fn touch_file(&mut self, path: &str) {
        if !self.files_touched.contains(&path.to_string()) {
            self.files_touched.push(path.to_string());
        }
        self.updated_at = chrono_now();
    }

    /// Generate a recap of where the user left off.
    pub fn recap(&self) -> String {
        let mut parts = Vec::new();

        if !self.context.is_empty() {
            parts.push(format!("Working on: {}", self.context));
        }

        if !self.files_touched.is_empty() {
            let files = if self.files_touched.len() <= 3 {
                self.files_touched.join(", ")
            } else {
                format!(
                    "{} and {} more",
                    self.files_touched.iter().take(3).cloned().collect::<Vec<_>>().join(", "),
                    self.files_touched.len() - 3
                )
            };
            parts.push(format!("Files: {files}"));
        }

        let recent: Vec<&SessionMessage> = self
            .messages
            .iter()
            .rev()
            .take(5)
            .collect();

        if !recent.is_empty() {
            let msgs: Vec<String> = recent
                .iter()
                .rev()
                .map(|m| format!("[{}] {}", m.role, truncate(&m.content, 80)))
                .collect();
            parts.push(format!("Recent: {}", msgs.join(" | ")));
        }

        if parts.is_empty() {
            "No session activity yet.".to_string()
        } else {
            parts.join(" — ")
        }
    }

    /// Markdown recap for export.
    pub fn to_markdown(&self) -> String {
        let mut md = format!(
            "# Session: {}\n\n**Created:** {}  \n**Updated:** {}  \n",
            self.name, self.created_at, self.updated_at
        );

        if !self.context.is_empty() {
            md.push_str(&format!("\n**Context:** {}\n", self.context));
        }

        if !self.files_touched.is_empty() {
            md.push_str("\n## Files\n");
            for f in &self.files_touched {
                md.push_str(&format!("- `{f}`\n"));
            }
        }

        md.push_str("\n## Messages\n");
        for msg in &self.messages {
            md.push_str(&format!(
                "- **[{}]** {}: {}\n",
                msg.timestamp, msg.role, msg.content
            ));
        }

        md
    }
}

pub struct SessionStore {
    dir: PathBuf,
    current: Session,
}

impl SessionStore {
    pub fn new() -> Self {
        let dir = session_dir();
        std::fs::create_dir_all(&dir).ok();
        let current = Self::load_latest(&dir).unwrap_or_else(|| Session::new("default"));
        Self { dir, current }
    }

    pub fn current(&self) -> &Session {
        &self.current
    }

    pub fn current_mut(&mut self) -> &mut Session {
        &mut self.current
    }

    pub fn save(&self) -> Result<(), String> {
        let path = self.dir.join(format!("{}.json", self.current.name));
        let json = serde_json::to_string_pretty(&self.current)
            .map_err(|e| format!("serialize: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("write: {e}"))?;
        Ok(())
    }

    pub fn list_sessions(&self) -> Vec<String> {
        let mut names = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry
                    .path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                {
                    names.push(name.to_string());
                }
            }
        }
        names.sort();
        names
    }

    fn load_latest(dir: &std::path::Path) -> Option<Session> {
        let mut sessions: Vec<(String, std::fs::Metadata)> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(meta) = path.metadata() {
                        if let Ok(modified) = meta.modified() {
                            sessions.push((
                                path.to_string_lossy().to_string(),
                                meta,
                            ));
                            // Use path for sorting; we'll just mark for later
                            let _ = modified;
                        }
                    }
                }
            }
        }
        // Sort by modification time, newest first
        sessions.sort_by(|(_, a), (_, b)| {
            b.modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .cmp(
                    &a.modified()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                )
        });

        sessions.first().and_then(|(path, _)| {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|json| serde_json::from_str(&json).ok())
        })
    }
}

fn session_dir() -> PathBuf {
    directories::ProjectDirs::from("com", "cortana", "cortana")
        .map(|dirs| dirs.data_dir().join("sessions"))
        .unwrap_or_else(|| PathBuf::from(".cortana/sessions"))
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len])
    }
}

fn chrono_now() -> String {
    // Simple UTC timestamp without pulling in chrono crate
    use std::time::SystemTime;
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(dur) => {
            let secs = dur.as_secs();
            // Rough: seconds since epoch -> human readable
            let days = secs / 86400;
            let time = secs % 86400;
            let hours = time / 3600;
            let minutes = (time % 3600) / 60;
            // Approximate date from unix epoch (works from 1970-2100)
            let year = 1970 + (days / 365) as i64; // crude, good enough for session stamps
            let day_of_year = days % 365;
            let month = (day_of_year / 30 + 1).min(12);
            let day = (day_of_year % 30 + 1).min(31);
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                year, month, day, hours, minutes, secs % 60
            )
        }
        Err(_) => "unknown".to_string(),
    }
}
