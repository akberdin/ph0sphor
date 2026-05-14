//! PHOSPHOR server entry point.
//!
//! Milestone 0: this binary only prints a banner and version info. Collectors,
//! state store and WebSocket endpoint land in Milestone 2.

use ph0sphor_core::{APP_VERSION, PROTOCOL_VERSION};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("ph0sphor-server {APP_VERSION} (protocol v{PROTOCOL_VERSION})");
        return;
    }

    let demo = args.iter().any(|a| a == "--demo");

    println!("PHOSPHOR server {APP_VERSION} (protocol v{PROTOCOL_VERSION})");
    if demo {
        println!("[demo] demo mode placeholder; real collectors land in Milestone 2.");
    } else {
        println!("Not implemented yet. See docs/roadmap.md for the milestone plan.");
    }
}
