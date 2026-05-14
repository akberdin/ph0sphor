//! Administrative CLI for PHOSPHOR.
//!
//! Milestone 0: prints banner and version. Pairing confirmation, token
//! management and demo data generation land alongside the server in
//! later milestones.

use ph0sphor_core::{APP_VERSION, PROTOCOL_VERSION};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("ph0sphorctl {APP_VERSION} (protocol v{PROTOCOL_VERSION})");
        return;
    }

    println!("PHOSPHOR ctl {APP_VERSION} (protocol v{PROTOCOL_VERSION})");
    println!("Subcommands: pair | status | validate-config | gen-demo  (all stubs).");
}
