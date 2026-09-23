use dictation_config::Config;
use std::collections::HashSet;
use std::process::Command;

/// Compare the configured Whisper model with candidate model files.
pub fn run(args: &[String], config: &Config) -> Result<String, String> {
    let default_wav = dictation_config::state_path()
        .with_file_name("dictation-bench.wav")
        .to_string_lossy()
        .into_owned();
    let mut wav = None;
    let mut reference = None;
    let mut models = Vec::new();
    let mut backends = Vec::new();
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--model" => {
                index += 1;
                let path = args.get(index).ok_or("--model requires a model path")?;
                models.push(path.clone());
            }
            "--backend" => {
                index += 1;
                let backend = args
                    .get(index)
                    .ok_or("--backend requires a name (moonshine or zipformer)")?;
                if !matches!(backend.as_str(), "moonshine" | "zipformer") {
                    return Err(format!(
                        "unsupported candidate backend `{backend}`; use moonshine or zipformer"
                    ));
                }
                backends.push(backend.clone());
            }
            "--reference" => {
                index += 1;
                reference = Some(
                    args.get(index)
                        .ok_or("--reference requires transcript text")?
                        .clone(),
                );
            }
            "--reference-file" => {
                index += 1;
                let path = args
                    .get(index)
                    .ok_or("--reference-file requires a file path")?;
                reference =
                    Some(std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?);
            }
            option if option.starts_with('-') => {
                return Err(format!("unknown benchmark option `{option}`"));
            }
            path if wav.is_none() => wav = Some(path.to_string()),
            path => return Err(format!("unexpected benchmark argument `{path}`")),
        }
        index += 1;
    }
    let wav = wav.unwrap_or(default_wav);

    let mut all_models = vec![config.engine.whisper_model.clone()];
    all_models.extend(models);
    let mut seen = HashSet::new();
    all_models.retain(|path| seen.insert(path.clone()));

    let executable =
        std::env::current_exe().map_err(|e| format!("locate dictation executable: {e}"))?;
    let mut report = format!("Dictation backend benchmark\naudio: {wav}\n");
    if reference.is_none() {
        report.push_str("quality: transcript shown for manual comparison; pass --reference or --reference-file for WER\n");
    }
    for model in all_models {
        let output = Command::new(&executable)
            .arg("__whisper-bench-one")
            .arg(&wav)
            .arg(&model)
            .arg(reference.as_deref().unwrap_or("-"))
            .output()
            .map_err(|e| format!("start benchmark for `{model}`: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "benchmark failed for `{model}`: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        report.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    let mut seen_backends = HashSet::new();
    backends.retain(|backend| seen_backends.insert(backend.clone()));
    for backend in backends {
        let output = Command::new(&executable)
            .arg("__streaming-bench-one")
            .arg(&wav)
            .arg(&backend)
            .arg(reference.as_deref().unwrap_or("-"))
            .output()
            .map_err(|e| format!("start benchmark for `{backend}`: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "benchmark failed for `{backend}`: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        report.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    Ok(report)
}
