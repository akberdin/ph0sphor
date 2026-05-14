//! PHOSPHOR client entry point: terminal setup + tokio runtime.

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ph0sphor_client::{
    app::{run, RunOptions},
    config::ClientConfig,
};
use ph0sphor_core::{APP_VERSION, PROTOCOL_VERSION};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("{msg}\n\n{HELP}");
            return ExitCode::FAILURE;
        }
    };
    match args {
        Args::Version => {
            println!("ph0sphor-client {APP_VERSION} (protocol v{PROTOCOL_VERSION})");
            ExitCode::SUCCESS
        }
        Args::Help => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        Args::Run {
            config,
            demo,
            server,
            token,
        } => {
            let cfg = match load_config(config.as_deref(), demo, server, token) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("config error: {e}");
                    return ExitCode::FAILURE;
                }
            };

            let rt = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("failed to start tokio runtime: {e}");
                    return ExitCode::FAILURE;
                }
            };

            match run_tui(&rt, cfg, RunOptions { demo }) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("client exited with error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

fn run_tui(
    rt: &tokio::runtime::Runtime,
    cfg: ClientConfig,
    options: RunOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    install_panic_hook();
    let mut terminal = init_terminal()?;
    let result = rt.block_on(run(&mut terminal, cfg, options));
    restore_terminal()?;
    Ok(result?)
}

fn init_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // EnableMouseCapture/DisableMouseCapture is symmetric; we do not act
    // on mouse events (README §6 / §15.1 — keyboard-first, no mouse
    // dependency), but capturing here prevents the terminal from acting
    // on stray clicks.
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        original_hook(info);
    }));
}

fn load_config(
    path: Option<&str>,
    demo: bool,
    server_override: Option<String>,
    token_override: Option<String>,
) -> Result<ClientConfig, Box<dyn std::error::Error>> {
    let mut cfg = match (path, demo) {
        (Some(p), _) => ClientConfig::load_from_path(p)?,
        (None, true) => ClientConfig::demo(),
        (None, false) => ClientConfig::default(),
    };
    if let Some(s) = server_override {
        cfg.client.server = s;
    }
    if let Some(t) = token_override {
        cfg.client.token = t;
    }
    Ok(cfg)
}

const HELP: &str = "\
PHOSPHOR client.

Usage:
  ph0sphor-client [OPTIONS]

Options:
  --config <path>    Load configuration from a TOML file.
  --server <url>     Override the server URL (e.g. ws://main-pc.local:7077/ws).
  --token <token>    Override the client token presented at handshake.
  --demo             Run in demo mode (no server, synthetic telemetry).
  -V, --version      Print version and exit.
  -h, --help         Print this help and exit.
";

#[derive(Debug)]
enum Args {
    Run {
        config: Option<String>,
        demo: bool,
        server: Option<String>,
        token: Option<String>,
    },
    Version,
    Help,
}

fn parse_args() -> Result<Args, String> {
    let mut args = std::env::args().skip(1);
    let mut config = None;
    let mut server = None;
    let mut token = None;
    let mut demo = false;

    while let Some(a) = args.next() {
        match a.as_str() {
            "--version" | "-V" => return Ok(Args::Version),
            "--help" | "-h" => return Ok(Args::Help),
            "--demo" => demo = true,
            "--config" => config = Some(args.next().ok_or("--config requires a path")?),
            "--server" => server = Some(args.next().ok_or("--server requires a URL")?),
            "--token" => token = Some(args.next().ok_or("--token requires a value")?),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(Args::Run {
        config,
        demo,
        server,
        token,
    })
}
