//! Administrative CLI for PHOSPHOR.

use ph0sphor_core::{APP_VERSION, PROTOCOL_VERSION};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("ph0sphorctl {APP_VERSION} (protocol v{PROTOCOL_VERSION})");
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return ExitCode::SUCCESS;
    }

    match args.first().map(String::as_str) {
        Some("pair") => match args.get(1).map(String::as_str) {
            Some("confirm") => match args.get(2) {
                Some(code) => pair_confirm(code, parse_server(&args)),
                None => {
                    eprintln!("usage: ph0sphorctl pair confirm <code> [--server URL]");
                    ExitCode::FAILURE
                }
            },
            _ => {
                eprintln!("usage: ph0sphorctl pair confirm <code>");
                ExitCode::FAILURE
            }
        },
        Some(other) => {
            eprintln!("unknown subcommand: {other}");
            print_help();
            ExitCode::FAILURE
        }
        None => {
            print_help();
            ExitCode::SUCCESS
        }
    }
}

fn print_help() {
    println!(
        "PHOSPHOR ctl {APP_VERSION} (protocol v{PROTOCOL_VERSION})

Usage:
  ph0sphorctl pair confirm <code> [--server http://127.0.0.1:7078]
  ph0sphorctl --version
  ph0sphorctl --help

Subcommands:
  pair confirm    Confirm a pairing code displayed by a connecting
                  client. Must run on the same host as the server; the
                  control endpoint is loopback-only by design."
    );
}

fn parse_server(args: &[String]) -> String {
    for w in args.windows(2) {
        if w[0] == "--server" {
            return w[1].clone();
        }
    }
    "http://127.0.0.1:7078".to_string()
}

fn pair_confirm(code: &str, server: String) -> ExitCode {
    let url = format!("{}/control/pair/confirm", server.trim_end_matches('/'));
    let body = format!("{{\"code\":\"{code}\"}}");
    match http_post(&url, &body) {
        Ok(resp) => {
            let (status, body) = resp;
            if !(200..300).contains(&status) {
                eprintln!("control endpoint returned HTTP {status}: {body}");
                return ExitCode::FAILURE;
            }
            if body.contains("\"ok\":true") {
                println!("pairing confirmed");
                ExitCode::SUCCESS
            } else {
                eprintln!("pairing rejected: {body}");
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("failed to contact control endpoint at {url}: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Minimal HTTP/1.1 POST client backed by `std::net::TcpStream`. Avoids
/// pulling reqwest/ureq for a one-route CLI.
fn http_post(url: &str, body: &str) -> std::io::Result<(u16, String)> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| std::io::Error::other("only http:// URLs supported"))?;
    let (hostport, path) = rest.split_once('/').unwrap_or((rest, ""));
    let req = format!(
        "POST /{path} HTTP/1.1\r\nHost: {hostport}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    let mut stream = TcpStream::connect_timeout(
        &hostport
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| std::io::Error::other("bad hostport"))?,
        Duration::from_secs(5),
    )?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(req.as_bytes())?;

    let mut buf = String::new();
    stream.read_to_string(&mut buf)?;

    // Parse status line "HTTP/1.1 200 OK".
    let status = buf
        .split('\n')
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    // Body is after the blank line "\r\n\r\n".
    let body = buf.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");
    Ok((status, body.to_string()))
}

// `to_socket_addrs` import (kept local to avoid polluting the module head).
use std::net::ToSocketAddrs;
