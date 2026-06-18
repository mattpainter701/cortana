mod agent;
mod config;
mod face;
mod renderer;
mod session;
mod speech;
mod summary;
mod tui;
mod visual;

use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("error: {error}");
        eprintln!("\nRun `cortana --help` for usage.");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    // Default: launch TUI
    if args.is_empty() {
        return launch_tui();
    }

    match args[0].as_str() {
        "--help" | "-h" | "help" => {
            print_help();
            Ok(())
        }
        "--version" | "-V" | "version" => {
            println!("cortana {VERSION}");
            Ok(())
        }
        "tui" | "interactive" => launch_tui(),
        "boot" => {
            print!("{}", visual::boot_splash());
            Ok(())
        }
        "appear" | "appearance" => {
            print!("{}", visual::appearance_splash());
            Ok(())
        }
        "session" => cmd_session(&args[1..]),
        "summarize" => cmd_summarize(&args[1..]),
        "recap" => cmd_recap(&args[1..]),
        "speak" => cmd_speak(&args[1..]),
        "daemon" => cmd_daemon(&args[1..]),
        "opencode" => cmd_opencode(&args[1..]),
        "config" => cmd_config(&args[1..]),
        other => Err(format!("unknown command '{other}'. Run `cortana --help`.")),
    }
}

// ── TUI ────────────────────────────────────────────────────────────

fn launch_tui() -> Result<(), String> {
    use std::io;

    let config = config::CortanaConfig::load();

    // Print boot splash briefly before TUI takes over
    print!("{}", visual::boot_splash());
    std::thread::sleep(std::time::Duration::from_millis(800));

    // Clear screen and enter alternate screen
    crossterm::terminal::enable_raw_mode().map_err(|e| format!("raw mode: {e}"))?;
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide,
    )
    .map_err(|e| format!("terminal setup: {e}"))?;

    let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
    let mut terminal =
        ratatui::Terminal::new(backend).map_err(|e| format!("terminal init: {e}"))?;

    let mut app = tui::TuiApp::new(config);
    let result = app.run(&mut terminal);

    // Restore terminal
    crossterm::terminal::disable_raw_mode().ok();
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show,
    )
    .ok();

    result.map_err(|e| format!("tui error: {e}"))
}

// ── CLI Commands ────────────────────────────────────────────────────

fn cmd_session(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("start") => {
            print!("{}", visual::session_start_banner());
            let store = session::SessionStore::new();
            match store.save() {
                Ok(()) => println!("\nSession saved."),
                Err(e) => println!("\nSession save failed: {e}"),
            }
            Ok(())
        }
        Some("list") => {
            let store = session::SessionStore::new();
            let sessions = store.list_sessions();
            if sessions.is_empty() {
                println!("No saved sessions.");
            } else {
                println!("Saved sessions:");
                for s in &sessions {
                    println!("  - {s}");
                }
            }
            Ok(())
        }
        Some("recap") => {
            let store = session::SessionStore::new();
            println!("{}", store.current().recap());
            Ok(())
        }
        Some("export") => {
            let store = session::SessionStore::new();
            println!("{}", store.current().to_markdown());
            Ok(())
        }
        Some(cmd) => Err(format!("unknown session subcommand '{cmd}'")),
        None => Err("session requires a subcommand: start, list, recap, export".into()),
    }
}

fn cmd_summarize(args: &[String]) -> Result<(), String> {
    let mut fast = false;
    let mut mode = summary::SummaryMode::Text;
    let mut json = false;
    let mut input_path = None;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--fast" => fast = true,
            "--json" => json = true,
            "--mode" => {
                i += 1;
                let val = args.get(i).ok_or("--mode requires a value")?;
                mode = summary::SummaryMode::parse(val)?;
            }
            flag if flag.starts_with('-') => return Err(format!("unknown flag '{flag}'")),
            path => {
                if input_path.is_some() {
                    return Err("only one input path allowed".into());
                }
                input_path = Some(path.to_string());
            }
        }
        i += 1;
    }

    if !fast {
        return Err("only --fast (no-token) summaries are supported".into());
    }

    let input = if let Some(ref path) = input_path {
        fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?
    } else {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("stdin: {e}"))?;
        if buf.trim().is_empty() {
            return Err("no input — pass a path or pipe text".into());
        }
        buf
    };

    let result = summary::summarize(&input, mode, input_path.as_deref());

    if json {
        println!("{}", result.to_json());
    } else {
        println!("{}", result.to_banner());
        if !result.keywords.is_empty() {
            println!("KEYWORDS: {}", result.keywords.join(", "));
        }
        println!("TOKEN_USAGE: {}", result.token_usage);
    }

    Ok(())
}

fn cmd_recap(args: &[String]) -> Result<(), String> {
    let mut session_name = "current".to_string();
    let mut format = "markdown".to_string();
    let mut speak = false;
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--session" => {
                i += 1;
                session_name = args.get(i).ok_or("--session requires a value")?.clone();
            }
            "--format" => {
                i += 1;
                format = args.get(i).ok_or("--format requires a value")?.clone();
            }
            "--speak" => speak = true,
            flag => return Err(format!("unknown recap flag '{flag}'")),
        }
        i += 1;
    }

    let store = session::SessionStore::new();
    let session = store.current();

    if format == "json" {
        println!(
            "{{\"session\":\"{}\",\"recap\":\"{}\",\"context\":\"{}\",\"files\":{:?},\"message_count\":{}}}",
            summary::json_escape(session_name.trim()),
            summary::json_escape(&session.recap()),
            summary::json_escape(&session.context),
            session.files_touched,
            session.messages.len()
        );
    } else {
        println!("{}", session.to_markdown());
    }

    if speak {
        let tts = speech::TtsEngine::new(speech::TtsProvider::System);
        tts.speak_bg(&session.recap());
        println!("\n(reading recap aloud...)");
    }

    Ok(())
}

fn cmd_speak(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("speak requires a message".into());
    }
    let message = args.join(" ");
    let tts = speech::TtsEngine::new(speech::TtsProvider::System);
    println!("Speaking: {message}");
    tts.speak_bg(&message);
    Ok(())
}

fn cmd_daemon(args: &[String]) -> Result<(), String> {
    let mut stdio = false;
    for arg in args {
        match arg.as_str() {
            "--stdio" => stdio = true,
            flag => return Err(format!("unknown daemon flag '{flag}'")),
        }
    }
    if !stdio {
        return Err("daemon currently requires --stdio".into());
    }

    let mut face_state = face::FaceState::default();
    let target = face::FaceParams::from_signals(0.15, 0.4, 0.2, 0.0);
    face_state.tick_toward(target, 0.016);

    print!("{}", visual::session_start_banner());
    println!("cortana daemon ready on stdio");
    println!("face_params: {}", face_state.params().to_protocol_line());
    Ok(())
}

fn cmd_config(args: &[String]) -> Result<(), String> {
    if args.first().map(String::as_str) == Some("init") {
        let path = args.get(1).map(PathBuf::from).unwrap_or_else(|| {
            directories::ProjectDirs::from("com", "cortana", "cortana")
                .map(|d| d.config_dir().join("config.toml"))
                .unwrap_or_else(|| PathBuf::from("cortana.toml"))
        });

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }

        config::CortanaConfig::save_default(&path)?;
        println!("Config written to {}", path.display());
        Ok(())
    } else {
        let cfg = config::CortanaConfig::load();
        println!(
            "{}",
            toml::to_string_pretty(&cfg).map_err(|e| format!("serialize: {e}"))?
        );
        Ok(())
    }
}

fn cmd_opencode(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("install") => {
            let mut target = PathBuf::from(".opencode/plugins/cortana.ts");
            let mut print_only = false;
            let mut i = 1;

            while i < args.len() {
                match args[i].as_str() {
                    "--global" => {
                        target = directories::BaseDirs::new()
                            .map(|d| d.config_dir().join("opencode/plugins/cortana.ts"))
                            .ok_or("could not resolve user config directory for --global")?;
                    }
                    "--project" => {
                        target = PathBuf::from(".opencode/plugins/cortana.ts");
                    }
                    "--path" => {
                        i += 1;
                        target = args
                            .get(i)
                            .map(PathBuf::from)
                            .ok_or("--path requires a destination file")?;
                    }
                    "--print" => print_only = true,
                    flag => return Err(format!("unknown opencode install flag '{flag}'")),
                }
                i += 1;
            }

            let plugin = include_str!("../integrations/opencode/cortana.ts");
            if print_only {
                print!("{plugin}");
                return Ok(());
            }

            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
            }
            fs::write(&target, plugin).map_err(|e| format!("write {}: {e}", target.display()))?;
            println!("OpenCode Cortana addon installed to {}", target.display());
            println!("Set CORTANA_BIN=/path/to/cortana if the binary is not on PATH.");
            Ok(())
        }
        Some("path") => {
            println!("integrations/opencode/cortana.ts");
            Ok(())
        }
        Some(cmd) => Err(format!("unknown opencode subcommand '{cmd}'")),
        None => Err("opencode requires a subcommand: install, path".into()),
    }
}

// ── Help ────────────────────────────────────────────────────────────

fn print_help() {
    println!(
        "Cortana {VERSION} — Terminal-first AI assistant\n\n\
USAGE:\n  \
  cortana                     Launch the interactive TUI\n  \
  cortana tui                 Same as above\n  \
  cortana boot                Show boot splash\n  \
  cortana session start       Show session start banner\n  \
  cortana session list        List saved sessions\n  \
  cortana session recap       Print current session recap\n  \
  cortana session export      Export session as markdown\n  \
  cortana summarize --fast [--mode text|code|diff] [--json] [path]\n  \
  cortana recap --session current [--format markdown|json] [--speak]\n  \
  cortana speak <message>\n  \
  cortana daemon --stdio\n  \
  cortana opencode install [--project|--global|--path file|--print]\n  \
  cortana config [init]       Show or create config file\n\n\
TUI KEYBINDINGS:\n  \
  /           Enter command mode\n  \
  Tab         Enter chat mode\n  \
  Esc         Cancel / quit\n  \
  Enter       Send message\n\n\
COMMANDS (in TUI, prefix with /):\n  \
  /help          Show commands\n  \
  /recap         Show session recap\n  \
  /summarize     Fast no-token summary\n  \
  /speak         Speak text aloud\n  \
  /context       Set/get session context\n  \
  /clear         Clear messages\n  \
  /quit          Exit\n\n\
API KEY:\n  \
  Set DEEPSEEK_API_KEY env var for LLM chat (optional).\n  \
  Cortana works fully offline for recaps, summaries, and speech.\n  \
  Default endpoint: https://api.deepseek.com/v1"
    );
}
