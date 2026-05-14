//! PHOSPHOR client entry point.
//!
//! Milestone 0: this binary only prints a banner and version info. The
//! WebSocket client, auth handshake and Ratatui-based TUI land in Milestone 3.

use ph0sphor_core::{APP_VERSION, PROTOCOL_VERSION};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("ph0sphor-client {APP_VERSION} (protocol v{PROTOCOL_VERSION})");
        return;
    }

    let demo = args.iter().any(|a| a == "--demo");

    println!("PHOSPHOR client {APP_VERSION} (protocol v{PROTOCOL_VERSION})");
    if demo {
        println!("[demo] demo mode placeholder; the TUI lands in Milestone 3.");
    } else {
        println!("Not implemented yet. See docs/roadmap.md for the milestone plan.");
    }
}
