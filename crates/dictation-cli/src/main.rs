//! Entry point for the `dictation` command.

mod commands;

use dictation_config::load_or_create;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config = load_or_create();
    std::process::exit(commands::run(&args, &config));
}
