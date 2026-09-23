use super::USAGE;
use dictation_config::{config_path, load_checked};

pub(super) fn run(args: &[String]) -> i32 {
    match args.get(2).map(String::as_str) {
        None => {
            println!("{}", config_path().display());
            0
        }
        Some("check") => match load_checked() {
            Err(error) => {
                eprintln!("dictation: {error}");
                1
            }
            Ok(config) => {
                let errors = config.validation_errors();
                if errors.is_empty() {
                    println!("configuration looks valid: {}", config_path().display());
                    0
                } else {
                    eprintln!("configuration problems in {}:", config_path().display());
                    for error in errors {
                        eprintln!("  - {error}");
                    }
                    1
                }
            }
        },
        Some(other) => {
            eprintln!("dictation: unknown config command `{other}`\n{USAGE}");
            2
        }
    }
}
