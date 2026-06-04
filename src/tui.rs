use crate::agent::LlmClient;
use crate::config::CortanaConfig;
use crate::face::{FaceParams, FaceState};
use crate::renderer::{FaceRenderer, RendererBackend};
use crate::session::SessionStore;
use crate::speech::{SpeechEvent, TtsEngine, TtsProvider};
use crate::summary::{self, SummaryMode};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::widgets::canvas::Canvas;
use ratatui::{Frame, Terminal};
use std::io;
use std::time::{Duration, Instant};

pub struct TuiApp {
    pub config: CortanaConfig,
    pub sessions: SessionStore,
    pub face_state: FaceState,
    pub face_renderer: FaceRenderer,
    pub tts: TtsEngine,
    pub llm: Option<LlmClient>,
    pub input: String,
    pub cursor: usize,
    pub messages: Vec<TuiMessage>,
    pub status: String,
    pub recap_banner: String,
    pub mode: InputMode,
    pub running: bool,
    pub speech_rx: Option<std::sync::mpsc::Receiver<SpeechEvent>>,
    pub current_speech_event: Option<SpeechEvent>,
    pub last_frame: Instant,
    pub idle_seconds: f32,
    pub thinking: bool,
}

#[derive(Debug, Clone)]
pub struct TuiMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Chat,
    Speaking,
}

impl TuiApp {
    pub fn new(config: CortanaConfig) -> Self {
        let backend = RendererBackend::detect();
        let sessions = SessionStore::new();
        let recap = sessions.current().recap();
        let tts = TtsEngine::new(TtsProvider::from_str(&config.speech.tts_provider));
        let llm = LlmClient::new(config.llm.clone());

        Self {
            config,
            sessions,
            face_state: FaceState::default(),
            face_renderer: FaceRenderer::new(backend),
            tts,
            llm,
            input: String::new(),
            cursor: 0,
            messages: Vec::new(),
            status: "Ready. Type /help for commands.".into(),
            recap_banner: if recap.is_empty() {
                "No session activity yet.".into()
            } else {
                recap
            },
            mode: InputMode::Normal,
            running: true,
            speech_rx: None,
            current_speech_event: None,
            last_frame: Instant::now(),
            idle_seconds: 0.0,
            thinking: false,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<impl ratatui::backend::Backend>) -> io::Result<()> {
        self.last_frame = Instant::now();
        while self.running {
            // Check for speech events
            self.poll_speech();

            // Update face animation
            let delta = self.last_frame.elapsed().as_secs_f32();
            self.last_frame = Instant::now();
            self.idle_seconds += delta;

            // Compute target face params based on current state
            let target = self.compute_face_target();
            self.face_state.tick_toward(target, delta);

            // Draw frame
            terminal.draw(|f| self.draw(f))?;

            // Handle input (non-blocking)
            if event::poll(Duration::from_millis(16)).unwrap_or(false) {
                if let Ok(event) = event::read() {
                    self.handle_event(event);
                }
            }
        }
        Ok(())
    }

    fn poll_speech(&mut self) {
        if let Some(ref rx) = self.speech_rx {
            if let Ok(event) = rx.try_recv() {
                match &event {
                    SpeechEvent::Started(_) => {
                        self.mode = InputMode::Speaking;
                        self.status = "Speaking...".into();
                    }
                    SpeechEvent::Ended => {
                        self.mode = InputMode::Normal;
                        self.status = "Ready.".into();
                        self.speech_rx = None;
                        self.current_speech_event = None;
                        return;
                    }
                    _ => {}
                }
                self.current_speech_event = Some(event);
            }
        }
    }

    fn compute_face_target(&self) -> FaceParams {
        if let Some(SpeechEvent::Amplitude { rms, .. }) = &self.current_speech_event {
            // During speech, drive face from audio amplitude
            let valence = self.face_state.params().valence;
            let arousal = 0.6;
            FaceParams::from_signals(*rms, valence, arousal, self.idle_seconds)
        } else if self.thinking {
            // Thinking: slightly furrowed, eyes narrow, subtle motion
            let _params = self.face_state.params();
            FaceParams::from_signals(0.05, -0.1, 0.4, self.idle_seconds)
        } else if self.mode == InputMode::Chat {
            // Chat mode: attentive
            FaceParams::from_signals(0.0, 0.2, 0.3, self.idle_seconds)
        } else {
            // Idle: subtle breathing, occasional eye movement
            FaceParams::from_signals(0.0, 0.0, 0.1, self.idle_seconds)
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let face_params = self.face_state.params();

        // Main layout: left (face) | right (content)
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(36),  // face panel
                Constraint::Min(40),     // content
            ])
            .split(f.area());

        // Face panel
        self.draw_face(f, main[0], &face_params);

        // Right side: recap banner + messages + input
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),   // recap banner
                Constraint::Min(5),      // messages
                Constraint::Length(3),   // input
            ])
            .split(main[1]);

        self.draw_recap_banner(f, right[0]);
        self.draw_messages(f, right[1]);
        self.draw_input(f, right[2]);

        // Status bar at the very bottom
        let status_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(f.area());

        self.draw_status(f, status_area[1]);
    }

    fn draw_face(&self, f: &mut Frame, area: Rect, params: &FaceParams) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Cortana ")
            .border_style(Style::default().fg(Color::Cyan));

        let canvas = Canvas::default()
            .block(block)
            .x_bounds([-50.0, 50.0])
            .y_bounds([-60.0, 60.0])
            .paint(|ctx| {
                self.face_renderer.paint_on_canvas(ctx, params);
            });

        f.render_widget(canvas, area);
    }

    fn draw_recap_banner(&self, f: &mut Frame, area: Rect) {
        let truncated = if self.recap_banner.len() > self.config.summary.banner_max_chars {
            format!(
                "{}…",
                &self.recap_banner[..self.config.summary.banner_max_chars]
            )
        } else {
            self.recap_banner.clone()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Recap ")
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let text = Paragraph::new(Line::from(Span::styled(
            truncated,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::DIM),
        )))
        .wrap(Wrap { trim: true });
        f.render_widget(text, inner);
    }

    fn draw_messages(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Messages ")
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        // Show last N messages that fit
        let max_msgs = inner.height as usize;
        let start = if self.messages.len() > max_msgs {
            self.messages.len() - max_msgs
        } else {
            0
        };

        for msg in self.messages.iter().skip(start) {
            let (role_color, role_prefix) = match msg.role.as_str() {
                "cortana" | "assistant" => (Color::Cyan, "Cortana"),
                "system" => (Color::Yellow, "SYSTEM"),
                _ => (Color::Green, "You"),
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{role_prefix}> "),
                    Style::default()
                        .fg(role_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    &msg.content,
                    Style::default().fg(Color::White),
                ),
            ]));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "Welcome to Cortana. Type a message or /help for commands.",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            )));
        }

        let text = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true });
        f.render_widget(text, inner);
    }

    fn draw_input(&self, f: &mut Frame, area: Rect) {
        let mode_label = match self.mode {
            InputMode::Normal => " NORMAL ",
            InputMode::Chat => " CHAT ",
            InputMode::Speaking => " SPEAKING ",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Input {mode_label}"))
            .border_style(match self.mode {
                InputMode::Chat => Style::default().fg(Color::Green),
                InputMode::Speaking => Style::default().fg(Color::Magenta),
                _ => Style::default().fg(Color::DarkGray),
            });

        let inner = block.inner(area);
        f.render_widget(block, area);

        let prompt = format!("> {}", self.input);
        let cursor_pos = self.cursor + 2; // account for "> "

        let text = Paragraph::new(Line::from(vec![
            Span::styled(
                &prompt[..cursor_pos.min(prompt.len())],
                Style::default().fg(Color::White),
            ),
            Span::styled(
                if cursor_pos < prompt.len() {
                    &prompt[cursor_pos..]
                } else {
                    ""
                },
                Style::default().fg(Color::White),
            ),
            Span::styled(
                " ",
                Style::default()
                    .bg(Color::White)
                    .fg(Color::Black),
            ),
        ]));
        f.render_widget(text, inner);
    }

    fn draw_status(&self, f: &mut Frame, area: Rect) {
        let backend_label = match self.face_renderer.backend() {
            RendererBackend::Canvas => "Canvas",
            RendererBackend::Kitty => "Kitty",
            RendererBackend::Sixel => "Sixel",
        };

        let llm_status = if self.llm.as_ref().map_or(false, |l| l.is_available()) {
            "LLM:on"
        } else {
            "LLM:off"
        };

        let status_line = Line::from(vec![
            Span::styled(
                format!(" {llm_status} "),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(
                format!("render:{backend_label} "),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            ),
            Span::styled(
                &self.status,
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                " │ Esc:quit /:cmd Tab:mode",
                Style::default()
                    .fg(Color::Rgb(60, 60, 60))
                    .add_modifier(Modifier::DIM),
            ),
        ]);

        let p = Paragraph::new(status_line);
        f.render_widget(p, area);
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                match self.mode {
                    InputMode::Normal => self.handle_normal_key(key.code),
                    InputMode::Chat => self.handle_chat_key(key.code),
                    InputMode::Speaking => {
                        // During speech, Esc cancels
                        if key.code == KeyCode::Esc {
                            self.speech_rx = None;
                            self.current_speech_event = None;
                            self.mode = InputMode::Normal;
                            self.status = "Speech cancelled.".into();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_normal_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => self.quit(),
            KeyCode::Tab => {
                self.mode = InputMode::Chat;
                self.status = "Chat mode — type your message, Enter to send, Esc to cancel.".into();
            }
            KeyCode::Char('/') => {
                // Command mode — input starts with /
                self.mode = InputMode::Chat;
                self.input.push('/');
                self.cursor = 1;
                self.status = "Command mode. /help, /recap, /summarize, /speak, /clear".into();
            }
            KeyCode::Char('q') => self.quit(),
            _ => {
                // Any other key in normal mode enters chat
                self.mode = InputMode::Chat;
                if let KeyCode::Char(c) = code {
                    self.input.push(c);
                    self.cursor = self.input.len();
                }
            }
        }
    }

    fn handle_chat_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.input.clear();
                self.cursor = 0;
                self.mode = InputMode::Normal;
                self.status = "Ready.".into();
            }
            KeyCode::Enter => {
                let message = self.input.trim().to_string();
                self.input.clear();
                self.cursor = 0;

                if message.is_empty() {
                    return;
                }

                self.handle_message(&message);
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.input.remove(self.cursor - 1);
                    self.cursor -= 1;
                }
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor < self.input.len() {
                    self.cursor += 1;
                }
            }
            KeyCode::Home => {
                self.cursor = 0;
            }
            KeyCode::End => {
                self.cursor = self.input.len();
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor, c);
                self.cursor += 1;
            }
            _ => {}
        }
    }

    fn handle_message(&mut self, message: &str) {
        let now = chrono_now();

        // Add user message
        self.messages.push(TuiMessage {
            role: "user".into(),
            content: message.to_string(),
            timestamp: now.clone(),
        });
        self.sessions.current_mut().add_message("user", message);

        // Process command or chat
        if message.starts_with('/') {
            self.handle_command(message);
        } else {
            self.handle_chat(message);
        }
    }

    fn handle_command(&mut self, command: &str) {
        let parts: Vec<&str> = command.splitn(2, ' ').collect();
        let cmd = parts[0].trim_start_matches('/');
        let args = parts.get(1).unwrap_or(&"");

        match cmd {
            "help" => {
                let help = "/help — show commands\n/recap — session recap\n/summarize <text> — fast summary\n/speak <text> — speak aloud\n/ask <question> — ask LLM (uses tokens)\n/context <text> — set session context\n/clear — clear messages\n/quit — exit";
                self.add_response("system", help);
            }
            "ask" => {
                if args.is_empty() {
                    self.add_response("system", "Usage: /ask <your question> — sends to LLM (uses tokens)");
                } else {
                    self.handle_ask(args);
                }
            }
            "recap" => {
                let recap = self.sessions.current().recap();
                self.recap_banner = recap.clone();
                self.add_response("system", &format!("Session recap:\n{recap}"));
                self.idle_seconds = 0.0;
            }
            "summarize" => {
                if args.is_empty() {
                    self.add_response("system", "Usage: /summarize <text to summarize>");
                } else {
                    let result = summary::summarize(args, SummaryMode::Text, None);
                    self.recap_banner = result.to_banner();
                    self.add_response("system", &format!(
                        "Summary (0 tokens): {}\nKeywords: {}",
                        result.summary,
                        result.keywords.join(", ")
                    ));
                }
                self.idle_seconds = 0.0;
            }
            "speak" => {
                if args.is_empty() {
                    self.add_response("system", "Usage: /speak <text to speak>");
                } else {
                    self.speech_rx = self.tts.speak(args);
                    self.add_response("system", &format!("Speaking: {args}"));
                }
                self.idle_seconds = 0.0;
            }
            "clear" => {
                self.messages.clear();
                self.status = "Messages cleared.".into();
            }
            "context" => {
                if args.is_empty() {
                    let ctx = &self.sessions.current().context;
                    if ctx.is_empty() {
                        self.add_response("system", "No context set. Use /context <description> to set.");
                    } else {
                        self.add_response("system", &format!("Current context: {ctx}"));
                    }
                } else {
                    self.sessions.current_mut().set_context(args);
                    self.add_response("system", &format!("Context set: {args}"));
                    self.recap_banner = self.sessions.current().recap();
                }
                self.idle_seconds = 0.0;
            }
            "quit" => {
                self.quit();
            }
            _ => {
                self.add_response("system", &format!("Unknown command: /{cmd}. Try /help."));
            }
        }
    }

    fn handle_chat(&mut self, message: &str) {
        // Offline-first: just respond locally. LLM only via /ask command.
        self.thinking = false;
        let response = self.local_response(message);
        self.add_response("cortana", &response);
        self.status = "Ready.".into();
    }

    fn handle_ask(&mut self, question: &str) {
        self.thinking = true;
        self.status = "Thinking...".into();

        if let Some(ref llm) = self.llm {
            if llm.is_available() {
                let history: Vec<crate::agent::ChatMessage> = self
                    .messages
                    .iter()
                    .map(|m| crate::agent::ChatMessage {
                        role: m.role.clone(),
                        content: m.content.clone(),
                    })
                    .collect();

                match llm.chat(question, &history) {
                    Some(response) => {
                        self.thinking = false;
                        self.add_response("cortana", &response);
                        self.status = "Ready.".into();
                        if self.config.speech.speak_by_default && response.len() < 200 {
                            self.speech_rx = self.tts.speak(&response);
                        }
                        return;
                    }
                    None => {
                        // LLM failed
                    }
                }
            }
        }

        self.thinking = false;
        self.add_response("cortana", "Sorry, I couldn't reach the LLM. Check your API key and connection. I can still do fast summaries and recaps offline.");
        self.status = "Ready (LLM error).".into();
    }

    fn local_response(&self, message: &str) -> String {
        let lower = message.to_lowercase();

        if lower.contains("hello") || lower.contains("hi ") || lower == "hi" {
            return "Hey there. I'm Cortana — your terminal companion. I track your session, summarize files, and keep you oriented. Type /help to see what I can do.".into();
        }
        if lower.contains("recap") || (lower.contains("where") && lower.contains("left off")) {
            return self.sessions.current().recap();
        }
        if lower.contains("what") && (lower.contains("doing") || lower.contains("working")) {
            let ctx = &self.sessions.current().context;
            if ctx.is_empty() {
                return "No context set yet. Use /context to tell me what you're working on.".into();
            }
            return format!("Current context: {ctx}");
        }

        // Default: extract keywords and summarize
        let result = summary::summarize(message, SummaryMode::Text, None);
        format!(
            "Here's what I got: {} | Keywords: {} | Use /recap for session status, /context to track your work, or /ask for AI help.",
            result.summary,
            result.keywords.join(", ")
        )
    }

    fn add_response(&mut self, role: &str, content: &str) {
        let now = chrono_now();
        self.messages.push(TuiMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: now,
        });
        self.sessions.current_mut().add_message(role, content);
        self.sessions.save().ok();
    }

    fn quit(&mut self) {
        self.running = false;
        // Save session before exit
        self.sessions.save().ok();
    }
}

fn chrono_now() -> String {
    use std::time::SystemTime;
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(dur) => {
            let secs = dur.as_secs();
            let days = secs / 86400;
            let time = secs % 86400;
            let hours = time / 3600;
            let minutes = (time % 3600) / 60;
            let year = 1970 + (days / 365) as i64;
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
