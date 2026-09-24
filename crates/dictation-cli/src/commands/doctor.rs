use dictation_config::Config;
use std::{path::Path, process::Command};

pub(super) fn run(cfg: &Config) -> i32 {
    let mut failed = false;
    let check = |label: &str, ok: bool, detail: &str, failed: &mut bool| {
        println!(
            "[{}] {}{}",
            if ok { "ok" } else { "issue" },
            label,
            if detail.is_empty() {
                String::new()
            } else {
                format!(": {detail}")
            }
        );
        *failed |= !ok;
    };
    check(
        "config",
        dictation_config::load_checked().is_ok(),
        &dictation_config::config_path().display().to_string(),
        &mut failed,
    );
    for issue in cfg.validation_errors() {
        println!("[issue] {issue}");
        failed = true;
    }
    if cfg.validation_errors().is_empty() {
        println!("[ok] active model files");
    }
    // Same provider preflight the daemon runs at startup: missing build support
    // or a device not visible to the process shows up before model loading.
    match dictation_native::provider_status(&cfg.engine) {
        Ok(status) => check("compute provider", true, &status, &mut failed),
        Err(issue) => check("compute provider", false, &issue, &mut failed),
    }
    if cfg.engine.model_management {
        for (label, program, arg) in [
            ("download tool", "curl", "--version"),
            ("tar archive tool", "tar", "--version"),
            ("zip archive tool", "unzip", "-v"),
        ] {
            let available = Command::new(program)
                .arg(arg)
                .output()
                .map(|result| result.status.success())
                .unwrap_or(false);
            check(
                label,
                available,
                &format!("install `{program}` to manage downloaded models"),
                &mut failed,
            );
        }
    }
    let audio = Path::new("/dev/snd").is_dir();
    check(
        "audio device interface",
        audio,
        "no /dev/snd interface found; verify the audio server and input device",
        &mut failed,
    );
    let typing = match cfg.engine.typing_backend.as_str() {
        "uinput" => Path::new("/dev/uinput").exists(),
        "wtype" => Command::new("wtype").arg("--version").output().is_ok(),
        "dotool" => Command::new("dotool").arg("--help").output().is_ok(),
        "ydotool" => Command::new("ydotool").arg("--help").output().is_ok(),
        "xdotool" => Command::new("xdotool").arg("--version").output().is_ok(),
        _ => false,
    };
    check(
        "typing backend",
        typing,
        &format!("{}", cfg.engine.typing_backend.as_str()),
        &mut failed,
    );
    if failed {
        println!(
            "Run `dictation config check` for config details, or `dictation model list` for models."
        );
        1
    } else {
        println!("diagnostics passed");
        0
    }
}
