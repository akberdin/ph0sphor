//! PHOSPHOR server entry point.

use ph0sphor_core::{APP_VERSION, PROTOCOL_VERSION};
use ph0sphor_server::{
    auth::{AuthConfig, TokenStore},
    collectors::{spawn_demo, spawn_real, Collectors},
    config::ServerConfig,
    control::serve_control,
    net::serve_with_perf,
    state::State,
};
use std::process::ExitCode;
use tracing::{error, info};

fn main() -> ExitCode {
    match parse_args() {
        Args::Version => {
            println!("ph0sphor-server {APP_VERSION} (protocol v{PROTOCOL_VERSION})");
            ExitCode::SUCCESS
        }
        Args::Help => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        Args::Run { config, demo } => {
            init_tracing();
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(2)
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("failed to start tokio runtime: {e}");
                    return ExitCode::FAILURE;
                }
            };
            match rt.block_on(run(config, demo)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    error!(error = %e, "server exited with error");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

const HELP: &str = "\
PHOSPHOR server.

Usage:
  ph0sphor-server [OPTIONS]

Options:
  --config <path>    Load configuration from a TOML file.
  --demo             Run in demo mode (synthetic telemetry, loopback bind).
  -V, --version      Print version and exit.
  -h, --help         Print this help and exit.
";

#[derive(Debug)]
enum Args {
    Run { config: Option<String>, demo: bool },
    Version,
    Help,
}

fn parse_args() -> Args {
    let mut args = std::env::args().skip(1);
    let mut config = None;
    let mut demo = false;

    while let Some(a) = args.next() {
        match a.as_str() {
            "--version" | "-V" => return Args::Version,
            "--help" | "-h" => return Args::Help,
            "--demo" => demo = true,
            "--config" => {
                config = args.next();
                if config.is_none() {
                    eprintln!("--config requires a path");
                    return Args::Help;
                }
            }
            other => {
                eprintln!("unknown argument: {other}");
                return Args::Help;
            }
        }
    }
    Args::Run { config, demo }
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,ph0sphor_server=info"));
    let _ = fmt().with_env_filter(filter).try_init();
}

async fn run(config: Option<String>, demo: bool) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = match (&config, demo) {
        (Some(path), _) => ServerConfig::load_from_path(path)?,
        (None, true) => ServerConfig::demo(),
        (None, false) => ServerConfig::default(),
    };

    let hostname = hostname_or_default(&cfg.server.name);
    let os = os_label();
    let state = State::new(hostname, os);

    let collectors: Collectors = if demo {
        info!("starting demo collectors");
        spawn_demo(state.clone())
    } else {
        spawn_real(state.clone(), &cfg.collectors)
    };

    let store = match cfg.security.token_store.as_deref() {
        Some(path) => TokenStore::load_or_create(path)?,
        None => TokenStore::in_memory(),
    };
    let auth = AuthConfig::build(&cfg.security, store);

    let mut control = serve_control(&cfg.server.control_bind, auth.clone()).await?;
    info!(addr = %control.local_addr, "control endpoint ready");

    let mut handle =
        serve_with_perf(&cfg.server.bind, state, auth, cfg.performance.clone()).await?;
    info!(addr = %handle.local_addr, "ph0sphor-server ready");

    tokio::signal::ctrl_c().await?;
    info!("ctrl-c received, shutting down");

    handle.shutdown();
    control.shutdown();
    collectors.shutdown();
    handle.join().await;
    control.join().await;
    collectors.join().await;
    Ok(())
}

fn hostname_or_default(fallback: &str) -> String {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| fallback.to_string())
}

fn os_label() -> String {
    let name = sysinfo::System::name().unwrap_or_else(|| "unknown".to_string());
    let version = sysinfo::System::os_version().unwrap_or_else(|| "?".to_string());
    format!("{name} {version}")
}
