use base64::Engine;
use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

const DEFAULT_CONFIG: &str = r##"{
  // paclarp passes every argument after `--` to pacman.
  "pacman": "/usr/bin/pacman",
    "stdout": {
    "prefix": "",
    "suffix": "",
    "color": "auto",
    "text_color": "",
    "highlights": [
      { "pattern": "^warning:", "color": "33" },
      { "pattern": "^error:", "color": "31;1" }
    ],
    "progress": { "enabled": true, "width": 30, "filled": "#", "empty": "-", "template": "[{bar}] {percent}%", "color": "36" },
    "kitty": { "enabled": false, "frames": [], "interval_ms": 120, "width": 32, "height": 32 },
    "rules": [
      { "pattern": "^(::|warning:|error:)", "replacement": "$0" }
    ]
  },
  "stderr": {
    "prefix": "",
    "suffix": "",
    "color": "auto",
    "rules": []
  },
  "ui": {
    "style": "minimal",
    "verbosity": 1,
    "animation": true,
    "fps": 12,
    "sound": false,
    "spinner": ["|", "/", "-", "\\"],
    "operation_icons": {},
    "theme": { "package": "36", "download": "34", "install": "32", "warning": "33;1", "error": "31;1", "border": "90", "status": "37" }
  }
}"##;

#[derive(Parser, Debug)]
#[command(name = "paclarp", about = "Configurable pacman output wrapper")]
struct Args {
    /// JSONC configuration file (defaults to ~/.config/paclarp/config.jsonc)
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// Print the built-in JSONC schema and exit
    #[arg(long)]
    print_default_config: bool,
    /// Print a shell alias that routes pacman through paclarp
    #[arg(long)]
    print_alias: bool,
    /// Arguments passed to pacman (put them after `--`)
    #[arg(last = true)]
    pacman_args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
struct Config {
    pacman: String,
    stdout: StreamConfig,
    stderr: StreamConfig,
    ui: UiConfig,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
struct UiConfig {
    style: String,
    verbosity: u8,
    animation: bool,
    fps: u8,
    sound: bool,
    spinner: Vec<String>,
    operation_icons: std::collections::BTreeMap<String, String>,
    theme: Theme,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
struct Theme {
    package: String,
    download: String,
    install: String,
    warning: String,
    error: String,
    border: String,
    status: String,
}
#[derive(Clone, Debug)]
enum Event {
    TransactionStarted,
    DownloadProgress { percent: usize },
    DownloadFinished,
    InstallationStarted,
    InstallationFinished,
    HookStarted,
    HookFinished,
    TransactionFinished,
    TransactionFailed,
    Text(String),
}
#[derive(Clone, Copy, Debug)]
struct Capabilities {
    kitty: bool,
    ansi: bool,
    tty: bool,
    ssh: bool,
    ci: bool,
}
fn capabilities() -> Capabilities {
    let tty = atty::is(atty::Stream::Stdout);
    Capabilities {
        kitty: std::env::var("TERM")
            .map(|v| v == "xterm-kitty")
            .unwrap_or(false),
        ansi: tty,
        tty,
        ssh: std::env::var_os("SSH_CONNECTION").is_some(),
        ci: std::env::var_os("CI").is_some(),
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
struct StreamConfig {
    prefix: String,
    suffix: String,
    color: String,
    text_color: String,
    highlights: Vec<Highlight>,
    rules: Vec<Rule>,
    progress: ProgressConfig,
    kitty: KittyConfig,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
struct Highlight {
    pattern: String,
    color: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
struct ProgressConfig {
    enabled: bool,
    width: usize,
    filled: String,
    empty: String,
    template: String,
    color: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
struct KittyConfig {
    enabled: bool,
    frames: Vec<PathBuf>,
    interval_ms: u64,
    width: u32,
    height: u32,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
struct Rule {
    pattern: String,
    replacement: String,
    color: Option<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            pacman: "/usr/bin/pacman".into(),
            stdout: StreamConfig {
                prefix: String::new(),
                suffix: String::new(),
                color: "auto".into(),
                text_color: String::new(),
                highlights: vec![],
                rules: vec![Rule {
                    pattern: "^(::|warning:|error:)".into(),
                    replacement: "$0".into(),
                    color: None,
                }],
                progress: ProgressConfig::default(),
                kitty: KittyConfig::default(),
            },
            stderr: StreamConfig::default(),
            ui: UiConfig::default(),
        }
    }
}
impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            prefix: String::new(),
            suffix: String::new(),
            color: "auto".into(),
            text_color: String::new(),
            highlights: vec![],
            rules: vec![],
            progress: ProgressConfig::default(),
            kitty: KittyConfig::default(),
        }
    }
}
impl Default for Highlight {
    fn default() -> Self {
        Self {
            pattern: ".*".into(),
            color: "".into(),
        }
    }
}
impl Default for UiConfig {
    fn default() -> Self {
        Self {
            style: "minimal".into(),
            verbosity: 1,
            animation: true,
            fps: 12,
            sound: false,
            spinner: vec!["|".into(), "/".into(), "-".into(), "\\".into()],
            operation_icons: std::collections::BTreeMap::new(),
            theme: Theme::default(),
        }
    }
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            package: "36".into(),
            download: "34".into(),
            install: "32".into(),
            warning: "33;1".into(),
            error: "31;1".into(),
            border: "90".into(),
            status: "37".into(),
        }
    }
}

fn event_for(line: &str) -> Event {
    let lower = line.to_ascii_lowercase();
    if lower.contains("failed") || lower.starts_with("error") {
        Event::TransactionFailed
    } else if lower.contains("downloading") || lower.contains(" %") {
        Event::DownloadProgress {
            percent: Regex::new(r"(\d{1,3})%")
                .ok()
                .and_then(|r| r.captures(line))
                .and_then(|c| c[1].parse().ok())
                .unwrap_or(0),
        }
    } else if lower.contains("installing") || lower.contains("upgrading") {
        Event::InstallationStarted
    } else if lower.contains("transaction complete") || lower.contains("finished") {
        Event::TransactionFinished
    } else if lower.contains("hook") {
        Event::HookStarted
    } else if lower.starts_with("::") {
        Event::TransactionStarted
    } else {
        Event::Text(line.into())
    }
}
impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            width: 30,
            filled: "#".into(),
            empty: "-".into(),
            template: "[{bar}] {percent}%".into(),
            color: "36".into(),
        }
    }
}
impl Default for KittyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            frames: vec![],
            interval_ms: 120,
            width: 32,
            height: 32,
        }
    }
}
impl Default for Rule {
    fn default() -> Self {
        Self {
            pattern: ".*".into(),
            replacement: "$0".into(),
            color: None,
        }
    }
}

fn config_path(cli: Option<PathBuf>) -> Option<PathBuf> {
    cli.or_else(|| {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .map(|p| p.join("paclarp/config.jsonc"))
    })
    .or_else(|| {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|p| p.join(".config/paclarp/config.jsonc"))
    })
}
fn load_config(path: Option<PathBuf>) -> Result<Config, Box<dyn std::error::Error>> {
    let Some(path) = path else {
        return Ok(Config::default());
    };
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(path)?;
    let stripped = json_comments::StripComments::new(text.as_bytes());
    Ok(serde_json::from_reader(stripped)?)
}
fn ensure_config(path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = path else { return Ok(()) };
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, DEFAULT_CONFIG)?;
    eprintln!("paclarp: created config at {}", path.display());
    eprintln!("To use paclarp as pacman, add this to your shell configuration:");
    eprintln!("  alias pacman='paclarp --'");
    Ok(())
}
fn ansi(code: &str, value: &str) -> String {
    if code.is_empty() {
        value.into()
    } else {
        format!("\x1b[{}m{}\x1b[0m", code, value)
    }
}
fn kitty_frame(cfg: &KittyConfig, index: usize) -> String {
    if !cfg.enabled || cfg.frames.is_empty() || !capabilities().kitty {
        return String::new();
    }
    let path = &cfg.frames[index % cfg.frames.len()];
    let Ok(data) = fs::read(path) else {
        return String::new();
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    format!(
        "\x1b_Ga=T,f=100,c={},r={},m=0;{}\x1b\\",
        cfg.width, cfg.height, encoded
    )
}
fn render(input: &[u8], cfg: &StreamConfig, ui: &UiConfig) -> Vec<u8> {
    let text = String::from_utf8_lossy(input);
    let is_tty = atty::is(atty::Stream::Stdout);
    let enabled = cfg.color == "always" || (cfg.color == "auto" && is_tty);
    let mut out = String::new();
    let progress_re = Regex::new(r"(\d{1,3})%").unwrap();
    let mut frame = 0usize;
    for line in text.split_inclusive(['\n', '\r']) {
        let delimiter = line.chars().last().filter(|c| *c == '\n' || *c == '\r');
        let mut value = line.trim_end_matches(['\n', '\r']).to_string();
        let event = event_for(&value);
        if ui.verbosity == 0 && matches!(event, Event::Text(_)) {
            continue;
        }
        let mut image = String::new();
        if cfg.progress.enabled {
            if let Some(cap) = progress_re.captures(&value) {
                if let Ok(percent) = cap[1].parse::<usize>() {
                    let filled = (cfg.progress.width * percent.min(100)) / 100;
                    let bar = format!(
                        "{}{}",
                        cfg.progress.filled.repeat(filled),
                        cfg.progress
                            .empty
                            .repeat(cfg.progress.width.saturating_sub(filled))
                    );
                    image = kitty_frame(&cfg.kitty, frame);
                    value = cfg
                        .progress
                        .template
                        .replace("{bar}", &bar)
                        .replace("{percent}", &percent.to_string())
                        .replace("{image}", "");
                }
            }
        }
        let mut color = None;
        for highlight in &cfg.highlights {
            if let Ok(re) = Regex::new(&highlight.pattern) {
                if re.is_match(&value) {
                    color = Some(highlight.color.clone());
                    break;
                }
            }
        }
        for rule in &cfg.rules {
            if let Ok(re) = Regex::new(&rule.pattern) {
                if re.is_match(&value) {
                    value = re
                        .replace_all(&value, rule.replacement.as_str())
                        .into_owned();
                    color = rule.color.clone();
                }
            }
        }
        if !image.is_empty() {
            frame += 1;
            out.push_str(&image);
        }
        let value = if ui.style == "compact" {
            value.trim().to_string()
        } else {
            value
        };
        let value = format!("{}{}{}", cfg.prefix, value, cfg.suffix);
        let rendered = if enabled {
            ansi(
                color
                    .as_deref()
                    .filter(|c| !c.is_empty())
                    .unwrap_or(&cfg.text_color),
                &value,
            )
        } else {
            value
        };
        out.push_str(&rendered);
        if let Some(d) = delimiter {
            out.push(d);
        }
    }
    out.into_bytes()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.print_default_config {
        println!("{}", DEFAULT_CONFIG);
        return Ok(());
    }
    if args.print_alias {
        println!("alias pacman='paclarp --'");
        return Ok(());
    }
    let path = config_path(args.config);
    ensure_config(path.clone())?;
    let cfg = load_config(path)?;
    let mut child = Command::new(&cfg.pacman)
        .args(&args.pacman_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    child.stdout.take().unwrap().read_to_end(&mut stdout)?;
    child.stderr.take().unwrap().read_to_end(&mut stderr)?;
    let status = child.wait()?;
    io::stdout().write_all(&render(&stdout, &cfg.stdout, &cfg.ui))?;
    io::stderr().write_all(&render(&stderr, &cfg.stderr, &cfg.ui))?;
    std::process::exit(status.code().unwrap_or(1));
}
