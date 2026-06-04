const CYAN: &str = "\x1b[38;5;45m";
const BLUE: &str = "\x1b[38;5;39m";
const DIM_BLUE: &str = "\x1b[38;5;25m";
const WHITE_BLUE: &str = "\x1b[38;5;159m";
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";

/// Minimal boot splash: a cyan activation ring and greeting.
pub fn boot_splash() -> String {
    colorize(
        &[
            "",
            "                 ███████                 ",
            "              ███       ███              ",
            "            ██             ██            ",
            "           ██               ██           ",
            "           ██               ██           ",
            "            ██             ██            ",
            "              ███       ███              ",
            "                 ███████                 ",
            "",
            "              Hi, I’m Cortana.           ",
            "",
        ]
        .join("\n"),
        CYAN,
    )
}

/// Pixelated, grainy hologram reveal shown when Cortana appears to speak.
pub fn appearance_splash() -> String {
    let lines = [
        "                    ·  .   ·        .                  ",
        "                 .      ░░▒▒▒▒░░      ·               ",
        "                    ░▒▓▓▓▓▓▓▓▓▓▓▒░                    ",
        "                  ░▒▓▓██▓▒░░▒▓██▓▒░                  ",
        "                 ░▓██▓░          ░▓██░                ",
        "               · ▒██▒   ░░▒▒▒▒░░   ▒██▒ ·             ",
        "                ▒██░  ░▒▓████▓▒░  ░██▒               ",
        "                ▓█▓   ▒██░  ░██▒   ▓█▓               ",
        "                ▒██░  ░▒▓████▓▒░  ░██▒               ",
        "                 ▒██▒    ░▒▒░    ▒██▒                ",
        "                  ░▓██▒░        ░▒██▓░                ",
        "                    ░▒▓██▓▒▒▒▒▓██▓▒░                  ",
        "                       ░▒▓████▓▒░                     ",
        "                         ░▒██▒░                       ",
        "               ░▒▒░       ▒██▒       ░▒▒░             ",
        "            ░▒▓████▓▒░  ░▒████▒░  ░▒▓████▓▒░          ",
        "          ░▓██▓▒░░▒▓██▓▓████████▓▓██▓▒░░▒▓██▓░        ",
        "         ▒██▒░      ░▒▓██▓▒░░▒▓██▓▒░      ░▒██▒       ",
        "        ░██▒    ░▒▓████▒░      ░▒████▓▒░    ▒██░      ",
        "        ▒██░ ░▒▓██▓▒░              ░▒▓██▓▒░ ░██▒      ",
        "        ▓██▓▓██▓▒░    ░░▒▒▓▓▒▒░░    ░▒▓██▓▓██▓       ",
        "        ░▒▓▓▒░     ░▒▓████████▓▒░     ░▒▓▓▒░         ",
        "          ·      ░▒▓██▓▒░    ░▒▓██▓▒░      ·         ",
        "                    ·   booting presence   ·          ",
    ];

    let mut output = String::new();
    for (index, line) in lines.iter().enumerate() {
        let color = match index % 4 {
            0 => DIM_BLUE,
            1 => BLUE,
            2 => CYAN,
            _ => WHITE_BLUE,
        };
        output.push_str(color);
        if index == lines.len() - 1 {
            output.push_str(DIM);
        }
        output.push_str(line);
        output.push_str(RESET);
        output.push('\n');
    }
    output
}

/// Session-start banner combines the boot ring and hologram reveal.
pub fn session_start_banner() -> String {
    format!(
        "{}{}\n{}{}\n{}",
        BOLD,
        boot_splash(),
        RESET,
        appearance_splash(),
        colorize(
            "Cortana presence online. Voice and recap channels standing by.\n",
            CYAN
        )
    )
}

fn colorize(text: &str, color: &str) -> String {
    format!("{color}{text}{RESET}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_splash_contains_required_greeting() {
        assert!(boot_splash().contains("Hi, I’m Cortana."));
    }

    #[test]
    fn appearance_splash_is_pixelated_and_blue() {
        let splash = appearance_splash();

        assert!(splash.contains("▓"));
        assert!(splash.contains("\x1b[38;5;45m") || splash.contains("\x1b[38;5;39m"));
    }

    #[test]
    fn session_banner_announces_presence() {
        assert!(session_start_banner().contains("Cortana presence online"));
    }
}
