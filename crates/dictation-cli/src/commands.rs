//! `dictation` — the command you bind to a key.
//!
//! One key does both: `dictation toggle` starts dictation when idle and
//! stops it when running. `begin`/`end` exist for scripting.

use dictation_config::Config;
use dictation_core::control;

mod benchmark;
mod config;
mod doctor;
mod model;
mod status;

const USAGE: &str = "\
Usage: dictation <command>

Commands:
  toggle    Start or stop dictation  (bind THIS to your key)
  begin     Start dictation
  end       Stop dictation
  status    Show current state
  config    Print the config file path; `config check` validates settings
  benchmark [wav] [--model PATH ...] [--backend moonshine|zipformer ...] [--reference TEXT]
            Compare recognition backends for speed, memory, and transcript quality
            (`--reference-file PATH` accepts a transcript file)
  benchmark-tune [wav] Compare encoder windows and thread counts for current model
  model <check|list|info|install|remove|set|recommend|benchmark|management> Manage recognition models
  doctor    Diagnose configuration, model, audio and typing setup
";

pub fn run(args: &[String], cfg: &Config) -> i32 {
    let cmd = args.get(1).map(String::as_str).unwrap_or("");

    // Hidden internal commands (used by the engine daemon and diagnostics).
    match cmd {
        "__native-run" => return dictation_native::run(cfg),
        "__whisper-bench-one" => {
            let Some(wav) = args.get(2) else {
                eprintln!("missing benchmark WAV path");
                return 2;
            };
            let Some(model) = args.get(3) else {
                eprintln!("missing benchmark model path");
                return 2;
            };
            let reference = args.get(4).map(String::as_str).filter(|text| *text != "-");
            return match dictation_native::whisper_bench_model(&cfg.engine, wav, model, reference) {
                Ok(output) => {
                    print!("{output}");
                    0
                }
                Err(error) => {
                    eprintln!("dictation benchmark failed: {error}");
                    1
                }
            };
        }
        "__streaming-bench-one" => {
            let Some(wav) = args.get(2) else {
                eprintln!("missing benchmark WAV path");
                return 2;
            };
            let Some(backend) = args.get(3) else {
                eprintln!("missing backend name");
                return 2;
            };
            let reference = args.get(4).map(String::as_str).filter(|text| *text != "-");
            return match dictation_native::streaming_bench_model(
                &cfg.engine,
                wav,
                backend,
                reference,
            ) {
                Ok(output) => {
                    print!("{output}");
                    0
                }
                Err(error) => {
                    eprintln!("dictation benchmark failed: {error}");
                    1
                }
            };
        }
        "__whisper-selftest" => {
            return match dictation_native::whisper_selftest(&cfg.engine) {
                Ok(text) => {
                    println!("whisper ok; transcribed {} bytes: {text:?}", text.len());
                    0
                }
                Err(e) => {
                    eprintln!("whisper selftest failed: {e}");
                    1
                }
            };
        }
        "benchmark" => {
            return match benchmark::run(args, cfg) {
                Ok(report) => {
                    print!("{report}");
                    0
                }
                Err(error) => {
                    eprintln!("dictation benchmark: {error}");
                    2
                }
            };
        }
        "model" => return model::run(args, cfg),
        "doctor" => return doctor::run(cfg),
        "benchmark-tune" | "__whisper-bench" => {
            let path = args.get(2).map_or_else(
                || {
                    dictation_config::state_path()
                        .with_file_name("dictation-bench.wav")
                        .to_string_lossy()
                        .into_owned()
                },
                String::clone,
            );
            match dictation_native::whisper_bench(&cfg.engine, &path) {
                Ok(report) => {
                    print!("{report}");
                    return 0;
                }
                Err(e) => {
                    eprintln!("dictation benchmark failed: {e}");
                    return 1;
                }
            }
        }
        "__type" => {
            return {
                let text = args.get(2).map(String::as_str).unwrap_or("");
                match dictation_native::type_text_once(&cfg.engine, text) {
                    Ok(()) => 0,
                    Err(e) => {
                        eprintln!("dictation __type: {e}");
                        1
                    }
                }
            };
        }
        _ => {}
    }

    match cmd {
        "toggle" => run_control("toggle", control::toggle(&cfg.engine)),
        "begin" | "start" => run_control("begin", control::begin(&cfg.engine)),
        "end" | "stop" => run_control("end", control::end()),
        "status" => status::run(cfg),
        "config" => config::run(args),
        "-h" | "--help" | "help" => {
            print!("{USAGE}");
            0
        }
        _ => {
            eprint!("{USAGE}");
            2
        }
    }
}

fn run_control(command: &str, result: std::io::Result<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("dictation: {command}: {error}");
            1
        }
    }
}
