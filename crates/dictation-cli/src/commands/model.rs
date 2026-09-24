use dictation_config::{Config, config_path};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

struct Entry {
    name: &'static str,
    backend: &'static str,
    file: &'static str,
    url: &'static str,
}
macro_rules! vosk_model {
    ($label:literal, $folder:literal) => {
        Entry {
            name: $label,
            backend: "vosk",
            file: $folder,
            url: concat!("https://alphacephei.com/vosk/models/", $folder, ".zip"),
        }
    };
}
const CATALOG: &[Entry] = &[
    Entry {
        name: "whisper-tiny.en",
        backend: "whisper",
        file: "ggml-tiny.en-q5_1.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en-q5_1.bin",
    },
    Entry {
        name: "whisper-base.en",
        backend: "whisper",
        file: "ggml-base.en-q5_1.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en-q5_1.bin",
    },
    Entry {
        name: "whisper-small.en",
        backend: "whisper",
        file: "ggml-small.en-q5_1.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en-q5_1.bin",
    },
    Entry {
        name: "whisper-tiny",
        backend: "whisper",
        file: "ggml-tiny-q5_1.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q5_1.bin",
    },
    Entry {
        name: "whisper-base",
        backend: "whisper",
        file: "ggml-base-q5_1.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin",
    },
    Entry {
        name: "whisper-small",
        backend: "whisper",
        file: "ggml-small-q5_1.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin",
    },
    Entry {
        name: "whisper-medium.en",
        backend: "whisper",
        file: "ggml-medium.en-q5_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en-q5_0.bin",
    },
    Entry {
        name: "whisper-medium",
        backend: "whisper",
        file: "ggml-medium-q5_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium-q5_0.bin",
    },
    Entry {
        name: "whisper-large-v2",
        backend: "whisper",
        file: "ggml-large-v2-q5_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v2-q5_0.bin",
    },
    Entry {
        name: "whisper-large-v3",
        backend: "whisper",
        file: "ggml-large-v3-q5_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin",
    },
    Entry {
        name: "whisper-large-v3-turbo",
        backend: "whisper",
        file: "ggml-large-v3-turbo-q5_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
    },
    Entry {
        name: "whisper-tiny.en-q8_0",
        backend: "whisper",
        file: "ggml-tiny.en-q8_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en-q8_0.bin",
    },
    Entry {
        name: "whisper-tiny-q8_0",
        backend: "whisper",
        file: "ggml-tiny-q8_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q8_0.bin",
    },
    Entry {
        name: "whisper-base.en-q8_0",
        backend: "whisper",
        file: "ggml-base.en-q8_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en-q8_0.bin",
    },
    Entry {
        name: "whisper-base-q8_0",
        backend: "whisper",
        file: "ggml-base-q8_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q8_0.bin",
    },
    Entry {
        name: "whisper-small.en-q8_0",
        backend: "whisper",
        file: "ggml-small.en-q8_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en-q8_0.bin",
    },
    Entry {
        name: "whisper-small-q8_0",
        backend: "whisper",
        file: "ggml-small-q8_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q8_0.bin",
    },
    Entry {
        name: "whisper-medium.en-q8_0",
        backend: "whisper",
        file: "ggml-medium.en-q8_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en-q8_0.bin",
    },
    Entry {
        name: "whisper-medium-q8_0",
        backend: "whisper",
        file: "ggml-medium-q8_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium-q8_0.bin",
    },
    Entry {
        name: "whisper-large-v3-turbo-q8_0",
        backend: "whisper",
        file: "ggml-large-v3-turbo-q8_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q8_0.bin",
    },
    Entry {
        name: "moonshine-tiny",
        backend: "moonshine",
        file: "sherpa-onnx-moonshine-tiny-en-int8",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-tiny-en-int8.tar.bz2",
    },
    Entry {
        name: "zipformer-20m",
        backend: "zipformer",
        file: "sherpa-onnx-streaming-zipformer-en-20M-2023-02-17",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-en-20M-2023-02-17.tar.bz2",
    },
    Entry {
        name: "sensevoice-multilingual-2025",
        backend: "sensevoice",
        file: "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2025-09-09.tar.bz2",
    },
    Entry {
        name: "sensevoice-multilingual-2024",
        backend: "sensevoice",
        file: "sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2024-07-17",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2024-07-17.tar.bz2",
    },
    Entry {
        name: "moonshine-base-en",
        backend: "moonshine",
        file: "sherpa-onnx-moonshine-base-en-int8",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-base-en-int8.tar.bz2",
    },
    Entry {
        name: "moonshine-v2-base-en",
        backend: "moonshine",
        file: "sherpa-onnx-moonshine-base-en-quantized-2026-02-27",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-base-en-quantized-2026-02-27.tar.bz2",
    },
    Entry {
        name: "moonshine-v2-base-es",
        backend: "moonshine",
        file: "sherpa-onnx-moonshine-base-es-quantized-2026-02-27",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-base-es-quantized-2026-02-27.tar.bz2",
    },
    Entry {
        name: "moonshine-v2-base-ar",
        backend: "moonshine",
        file: "sherpa-onnx-moonshine-base-ar-quantized-2026-02-27",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-base-ar-quantized-2026-02-27.tar.bz2",
    },
    Entry {
        name: "moonshine-v2-base-zh",
        backend: "moonshine",
        file: "sherpa-onnx-moonshine-base-zh-quantized-2026-02-27",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-base-zh-quantized-2026-02-27.tar.bz2",
    },
    Entry {
        name: "moonshine-v2-base-ja",
        backend: "moonshine",
        file: "sherpa-onnx-moonshine-base-ja-quantized-2026-02-27",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-base-ja-quantized-2026-02-27.tar.bz2",
    },
    Entry {
        name: "moonshine-v2-base-ko",
        backend: "moonshine",
        file: "sherpa-onnx-moonshine-base-ko-quantized-2026-02-27",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-base-ko-quantized-2026-02-27.tar.bz2",
    },
    Entry {
        name: "moonshine-v2-base-uk",
        backend: "moonshine",
        file: "sherpa-onnx-moonshine-base-uk-quantized-2026-02-27",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-base-uk-quantized-2026-02-27.tar.bz2",
    },
    Entry {
        name: "moonshine-v2-base-vi",
        backend: "moonshine",
        file: "sherpa-onnx-moonshine-base-vi-quantized-2026-02-27",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-moonshine-base-vi-quantized-2026-02-27.tar.bz2",
    },
    vosk_model!("vosk-en-us-small", "vosk-model-small-en-us-0.15"),
    vosk_model!("vosk-en-us", "vosk-model-en-us-0.22"),
    vosk_model!("vosk-en-us-lgraph", "vosk-model-en-us-0.22-lgraph"),
    vosk_model!("vosk-en-us-gigaspeech", "vosk-model-en-us-0.42-gigaspeech"),
    vosk_model!("vosk-en-in-small", "vosk-model-small-en-in-0.4"),
    vosk_model!("vosk-en-in", "vosk-model-en-in-0.5"),
    vosk_model!("vosk-ru-small", "vosk-model-small-ru-0.22"),
    vosk_model!("vosk-ru", "vosk-model-ru-0.42"),
    vosk_model!("vosk-fr-small", "vosk-model-small-fr-0.22"),
    vosk_model!("vosk-fr", "vosk-model-fr-0.22"),
    vosk_model!("vosk-de-small", "vosk-model-small-de-0.15"),
    vosk_model!("vosk-de", "vosk-model-de-0.21"),
    vosk_model!("vosk-es-small", "vosk-model-small-es-0.42"),
    vosk_model!("vosk-es", "vosk-model-es-0.42"),
    vosk_model!("vosk-pt-small", "vosk-model-small-pt-0.3"),
    vosk_model!("vosk-tr-small", "vosk-model-small-tr-0.3"),
    vosk_model!("vosk-vn-small", "vosk-model-small-vn-0.4"),
    vosk_model!("vosk-vn", "vosk-model-vn-0.4"),
    vosk_model!("vosk-it-small", "vosk-model-small-it-0.22"),
    vosk_model!("vosk-it", "vosk-model-it-0.22"),
    vosk_model!("vosk-nl-small", "vosk-model-small-nl-0.22"),
    vosk_model!("vosk-ca-small", "vosk-model-small-ca-0.4"),
    vosk_model!("vosk-ar", "vosk-model-ar-mgb2-0.4"),
    vosk_model!("vosk-fa-small", "vosk-model-small-fa-0.42"),
    vosk_model!("vosk-hi-small", "vosk-model-small-hi-0.22"),
    vosk_model!("vosk-hi", "vosk-model-hi-0.22"),
    vosk_model!("vosk-ja-small", "vosk-model-small-ja-0.22"),
    vosk_model!("vosk-ja", "vosk-model-ja-0.22"),
    vosk_model!("vosk-zh-small", "vosk-model-small-cn-0.22"),
    vosk_model!("vosk-zh", "vosk-model-cn-0.22"),
    vosk_model!("vosk-pl-small", "vosk-model-small-pl-0.22"),
    vosk_model!("vosk-ko-small", "vosk-model-small-ko-0.22"),
    vosk_model!("vosk-kz-small", "vosk-model-small-kz-0.42"),
    vosk_model!("vosk-eo-small", "vosk-model-small-eo-0.42"),
    vosk_model!("vosk-cs-small", "vosk-model-small-cs-0.4-rhasspy"),
    vosk_model!("vosk-uz-small", "vosk-model-small-uz-0.22"),
    vosk_model!("vosk-gu-small", "vosk-model-small-gu-0.42"),
    vosk_model!("vosk-tg-small", "vosk-model-small-tg-0.22"),
    vosk_model!("vosk-te-small", "vosk-model-small-te-0.42"),
    vosk_model!("vosk-ky-small", "vosk-model-small-ky-0.42"),
    vosk_model!("vosk-ka-small", "vosk-model-small-ka-0.42"),
];

fn root() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("dictation/models")
}
fn model_path(cfg: &Config, entry: &Entry) -> PathBuf {
    let _ = cfg;
    root().join(entry.file)
}
fn known(name: &str) -> Option<&'static Entry> {
    CATALOG.iter().find(|e| e.name == name)
}
fn save(cfg: &Config) -> Result<(), String> {
    let path = config_path();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let content = toml::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    let temp = path.with_extension("toml.tmp");
    std::fs::write(&temp, content).map_err(|e| e.to_string())?;
    std::fs::rename(temp, path).map_err(|e| e.to_string())
}
fn valid(entry: &Entry, path: &Path) -> bool {
    match entry.backend {
        "whisper" => {
            path.is_file()
                && std::fs::metadata(path)
                    .map(|m| m.len() >= whisper_min_bytes(entry.file))
                    .unwrap_or(false)
        }
        "vosk" => path.join("am/final.mdl").is_file() && path.join("conf/model.conf").is_file(),
        "moonshine" if path.join("encoder_model.ort").is_file() => {
            path.join("decoder_model_merged.ort").is_file() && path.join("tokens.txt").is_file()
        }
        "sensevoice" => path.join("model.int8.onnx").is_file() && path.join("tokens.txt").is_file(),
        "moonshine" => [
            "preprocess.onnx",
            "encode.int8.onnx",
            "uncached_decode.int8.onnx",
            "cached_decode.int8.onnx",
            "tokens.txt",
        ]
        .iter()
        .all(|f| path.join(f).is_file()),
        _ => [
            "encoder-epoch-99-avg-1.int8.onnx",
            "decoder-epoch-99-avg-1.onnx",
            "joiner-epoch-99-avg-1.int8.onnx",
            "tokens.txt",
        ]
        .iter()
        .all(|f| path.join(f).is_file()),
    }
}

fn whisper_min_bytes(file: &str) -> u64 {
    let mib = if file.contains("large-v3-turbo-q5_0") {
        500
    } else if file.contains("large-v3-turbo-q8_0") {
        750
    } else if file.contains("large-v2-q5_0") || file.contains("large-v3-q5_0") {
        900
    } else if file.contains("medium") && file.contains("q5_0") {
        450
    } else if file.contains("medium") && file.contains("q8_0") {
        700
    } else if file.contains("small") && file.contains("q5_1") {
        160
    } else if file.contains("small") && file.contains("q8_0") {
        210
    } else if file.contains("base") && file.contains("q5_1") {
        48
    } else if file.contains("base") && file.contains("q8_0") {
        65
    } else if file.contains("tiny") && file.contains("q5_1") {
        25
    } else if file.contains("tiny") && file.contains("q8_0") {
        35
    } else {
        1
    };
    mib * 1024 * 1024
}
fn download(entry: &Entry, dest: &Path) -> Result<(), String> {
    if valid(entry, dest) {
        return Ok(());
    }
    std::fs::create_dir_all(dest.parent().ok_or("invalid model destination")?)
        .map_err(|e| e.to_string())?;
    if entry.backend == "whisper" {
        let status = Command::new("curl")
            .args(["-fL", "--retry", "3", "-C", "-"])
            .arg(&entry.url)
            .arg("-o")
            .arg(dest)
            .status()
            .map_err(|e| format!("start curl: {e}"))?;
        if !status.success() {
            return Err(format!(
                "download failed for {}; rerun install to resume",
                entry.name
            ));
        }
    } else if entry.backend == "vosk" {
        let archive = dest.with_extension("zip");
        let status = Command::new("curl")
            .args(["-fL", "--retry", "3", "-C", "-"])
            .arg(&entry.url)
            .arg("-o")
            .arg(&archive)
            .status()
            .map_err(|e| format!("start curl: {e}"))?;
        if !status.success() {
            return Err(format!(
                "download failed for {}; rerun install to resume",
                entry.name
            ));
        }
        let status = Command::new("unzip")
            .arg("-oq")
            .arg(&archive)
            .arg("-d")
            .arg(dest.parent().unwrap())
            .status()
            .map_err(|e| format!("start unzip (install unzip): {e}"))?;
        if !status.success() {
            return Err(format!(
                "could not unpack {}; archive kept at {}",
                entry.name,
                archive.display()
            ));
        }
    } else {
        let archive = dest.with_extension("tar.bz2");
        let status = Command::new("curl")
            .args(["-fL", "--retry", "3", "-C", "-"])
            .arg(&entry.url)
            .arg("-o")
            .arg(&archive)
            .status()
            .map_err(|e| format!("start curl: {e}"))?;
        if !status.success() {
            return Err(format!(
                "download failed for {}; rerun install to resume",
                entry.name
            ));
        }
        let status = Command::new("tar")
            .arg("-xjf")
            .arg(&archive)
            .arg("-C")
            .arg(dest.parent().unwrap())
            .status()
            .map_err(|e| format!("start tar: {e}"))?;
        if !status.success() {
            return Err(format!(
                "could not unpack {}; archive kept at {}",
                entry.name,
                archive.display()
            ));
        }
    }
    if !valid(entry, dest) {
        return Err(format!(
            "downloaded files for {} are incomplete; rerun install to resume",
            entry.name
        ));
    }
    Ok(())
}

fn language(entry: &Entry) -> &'static str {
    match entry.backend {
        "whisper" if entry.name.contains(".en") => "English only",
        "whisper" => "Multilingual",
        "vosk" => {
            let name = entry.name;
            if name.starts_with("vosk-en-in") {
                "English (India)"
            } else if name.starts_with("vosk-en") {
                "English (US)"
            } else if name.starts_with("vosk-ru") {
                "Russian"
            } else if name.starts_with("vosk-fr") {
                "French"
            } else if name.starts_with("vosk-de") {
                "German"
            } else if name.starts_with("vosk-es") {
                "Spanish"
            } else if name.starts_with("vosk-pt") {
                "Portuguese"
            } else if name.starts_with("vosk-tr") {
                "Turkish"
            } else if name.starts_with("vosk-vn") {
                "Vietnamese"
            } else if name.starts_with("vosk-it") {
                "Italian"
            } else if name.starts_with("vosk-nl") {
                "Dutch"
            } else if name.starts_with("vosk-ca") {
                "Catalan"
            } else if name.starts_with("vosk-ar") {
                "Arabic"
            } else if name.starts_with("vosk-fa") {
                "Persian"
            } else if name.starts_with("vosk-hi") {
                "Hindi"
            } else if name.starts_with("vosk-ja") {
                "Japanese"
            } else if name.starts_with("vosk-zh") {
                "Chinese"
            } else if name.starts_with("vosk-pl") {
                "Polish"
            } else if name.starts_with("vosk-ko") {
                "Korean"
            } else if name.starts_with("vosk-kz") {
                "Kazakh"
            } else if name.starts_with("vosk-eo") {
                "Esperanto"
            } else if name.starts_with("vosk-cs") {
                "Czech"
            } else if name.starts_with("vosk-uz") {
                "Uzbek"
            } else if name.starts_with("vosk-gu") {
                "Gujarati"
            } else if name.starts_with("vosk-tg") {
                "Tajik"
            } else if name.starts_with("vosk-te") {
                "Telugu"
            } else if name.starts_with("vosk-ky") {
                "Kyrgyz"
            } else if name.starts_with("vosk-ka") {
                "Georgian"
            } else {
                "See model name"
            }
        }
        "moonshine" if entry.name.ends_with("-en") => "English",
        "moonshine" if entry.name.ends_with("-es") => "Spanish",
        "moonshine" if entry.name.ends_with("-ar") => "Arabic",
        "moonshine" if entry.name.ends_with("-zh") => "Chinese",
        "moonshine" if entry.name.ends_with("-ja") => "Japanese",
        "moonshine" if entry.name.ends_with("-ko") => "Korean",
        "moonshine" if entry.name.ends_with("-uk") => "Ukrainian",
        "moonshine" if entry.name.ends_with("-vi") => "Vietnamese",
        "moonshine" => "English",
        "sensevoice" => "English, Chinese, Japanese, Korean, Cantonese",
        "zipformer" => "English",
        _ => "Model specific",
    }
}

fn download_size(entry: &Entry) -> &'static str {
    if entry.backend == "whisper" {
        let file = entry.file;
        if file.contains("large-v3-turbo-q5_0") {
            "~547 MiB"
        } else if file.contains("large-v3-turbo-q8_0") {
            "~834 MiB"
        } else if file.contains("large-v2-q5_0") || file.contains("large-v3-q5_0") {
            "~1.03 GiB"
        } else if file.contains("medium") && file.contains("q5_0") {
            "~514 MiB"
        } else if file.contains("medium") && file.contains("q8_0") {
            "~786 MiB"
        } else if file.contains("small") && file.contains("q5_1") {
            "~181 MiB"
        } else if file.contains("small") && file.contains("q8_0") {
            "~252 MiB"
        } else if file.contains("base") && file.contains("q5_1") {
            "~57 MiB"
        } else if file.contains("base") && file.contains("q8_0") {
            "~78 MiB"
        } else if file.contains("tiny") && file.contains("q5_1") {
            "~31 MiB"
        } else if file.contains("tiny") && file.contains("q8_0") {
            "~44 MiB"
        } else {
            "varies"
        }
    } else if entry.backend == "vosk" {
        if entry.file == "vosk-model-vn-0.4" {
            "~80 MiB"
        } else if entry.file == "vosk-model-ar-mgb2-0.4" {
            "~320 MiB"
        } else if entry.file.contains("lgraph") {
            "~100–130 MiB"
        } else if entry.file.contains("small-") {
            "~30–90 MiB"
        } else {
            "~0.7–4.4 GiB"
        }
    } else if entry.backend == "moonshine" && entry.name.starts_with("moonshine-v2") {
        "~60–140 MiB"
    } else if entry.backend == "moonshine" && entry.name == "moonshine-tiny" {
        "~235 MiB"
    } else if entry.backend == "moonshine" {
        "~550 MiB"
    } else if entry.backend == "sensevoice" {
        "~230 MiB"
    } else {
        "varies by package"
    }
}

fn summary(entry: &Entry) -> &'static str {
    match entry.backend {
        "whisper" if entry.name.contains("large-v3-turbo") => {
            "Large-v3 Turbo; multilingual, faster large-model option"
        }
        "whisper" if entry.name.contains("large") => {
            "Large multilingual model; highest resource demand"
        }
        "whisper" if entry.name.contains("medium") => {
            "Medium model; balance of recognition and resource use"
        }
        "whisper" if entry.name.contains("small") => {
            "Small model; balanced quality and resource use"
        }
        "whisper" if entry.name.contains("base") => "Base model; moderate resource use",
        "whisper" => "Tiny model; lowest resource use",
        "vosk" if entry.file.contains("small-") => "Small-footprint streaming model",
        "vosk" if entry.file.contains("lgraph") => "Medium model with dynamic graph",
        "vosk" => "Larger-vocabulary model; higher disk and memory use",
        "moonshine" if entry.name.starts_with("moonshine-v2") => {
            "Newer language-specific Moonshine Base model"
        }
        "moonshine" if entry.name.contains("base") => "Moonshine Base; larger English model",
        "moonshine" => "Moonshine Tiny; compact English model",
        "sensevoice" => "Multilingual SenseVoiceSmall; automatic language recognition",
        "zipformer" => "Streaming Zipformer transducer",
        _ => "Speech recognition model",
    }
}

fn is_active(cfg: &Config, entry: &Entry, path: &Path) -> bool {
    cfg.engine.backend == entry.backend
        && match entry.backend {
            "whisper" => Path::new(&cfg.engine.whisper_model) == path,
            "moonshine" => Path::new(&cfg.engine.moonshine_model_dir) == path,
            "zipformer" => Path::new(&cfg.engine.zipformer_model_dir) == path,
            "vosk" => Path::new(&cfg.engine.model_path) == path,
            "sensevoice" => Path::new(&cfg.engine.sense_voice_model_dir) == path,
            _ => false,
        }
}

pub(super) fn run(args: &[String], cfg: &Config) -> i32 {
    if args.get(2).map(String::as_str) == Some("management") {
        let mut next = cfg.clone();
        match args.get(3).map(String::as_str) {
            Some("on") => next.engine.model_management = true,
            Some("off") => next.engine.model_management = false,
            _ => {
                eprintln!("usage: dictation model management <on|off>");
                return 2;
            }
        }
        return match save(&next) {
            Ok(()) => {
                println!(
                    "model management {}",
                    if next.engine.model_management {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                0
            }
            Err(e) => {
                eprintln!("dictation model: {e}");
                1
            }
        };
    }
    if !cfg.engine.model_management
        && matches!(
            args.get(2).map(String::as_str),
            Some("install" | "set" | "remove")
        )
    {
        eprintln!("model management is disabled; run `dictation model management on` first");
        return 1;
    }
    let result = match args.get(2).map(String::as_str).unwrap_or("list") {
        "list" => {
            let mut backend_filter = None;
            let mut installed_only = false;
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--backend" => {
                        i += 1;
                        let Some(value) = args.get(i) else {
                            eprintln!(
                                "--backend requires whisper, vosk, moonshine, zipformer, or sensevoice"
                            );
                            return 2;
                        };
                        backend_filter = Some(value.as_str());
                    }
                    "--installed" => installed_only = true,
                    option => {
                        eprintln!(
                            "unknown model list option `{option}`; use --backend NAME or --installed"
                        );
                        return 2;
                    }
                }
                i += 1;
            }
            println!(
                "MODEL                              FAMILY       LANGUAGE                                      DOWNLOAD       STATUS       NEXT COMMAND"
            );
            println!(
                "Use `dictation model info NAME` for the model path, source, and install/set/remove commands.\n"
            );
            let mut count = 0;
            let mut entries: Vec<_> = CATALOG
                .iter()
                .filter(|e| backend_filter.is_none_or(|b| e.backend == b))
                .collect();
            entries.sort_by_key(|e| (e.backend, e.name));
            for e in entries {
                let p = model_path(cfg, e);
                let installed = valid(e, &p);
                if installed_only && !installed {
                    continue;
                }
                let active = is_active(cfg, e, &p);
                let status = if active {
                    "active"
                } else if installed {
                    "installed"
                } else {
                    "missing"
                };
                let action = if active {
                    "already the default".to_string()
                } else if installed {
                    format!("dictation model set {}", e.name)
                } else {
                    format!("dictation model install {}", e.name)
                };
                println!(
                    "{:<34} {:<12} {:<45} {:<14} {:<12} {}",
                    e.name,
                    e.backend,
                    language(e),
                    download_size(e),
                    status,
                    action
                );
                println!("  {}", summary(e));
                count += 1;
            }
            println!(
                "\n{count} models shown. Filter with `--backend NAME`; show local models only with `--installed`."
            );
            return 0;
        }
        "info" => {
            let Some(name) = args.get(3) else {
                eprintln!("usage: dictation model info <name>");
                return 2;
            };
            let Some(e) = known(name) else {
                eprintln!("unknown model `{name}`; run dictation model list");
                return 2;
            };
            let path = model_path(cfg, e);
            let installed = valid(e, &path);
            let active = is_active(cfg, e, &path);
            println!(
                "Name:        {}\nFamily:      {}\nLanguage:    {}\nDownload:    {}\nDescription: {}\nStatus:      {}\nManaged at:  {}\nSource:      {}",
                e.name,
                e.backend,
                language(e),
                download_size(e),
                summary(e),
                if active {
                    "active"
                } else if installed {
                    "installed"
                } else {
                    "not installed"
                },
                path.display(),
                e.url
            );
            println!("Install:     dictation model install {}", e.name);
            println!("Select:      dictation model set {}", e.name);
            if active {
                println!("Remove:      select another model first");
            } else {
                println!("Remove:      dictation model remove {}", e.name);
            }
            return 0;
        }
        "check" => {
            if let Some(name) = args.get(3) {
                let Some(e) = known(name) else {
                    eprintln!("unknown catalog model `{name}`");
                    return 2;
                };
                let p = model_path(cfg, e);
                if valid(e, &p) {
                    println!(
                        "{} is installed and its required files are present: {}",
                        e.name,
                        p.display()
                    );
                    return 0;
                }
                eprintln!("{} is missing or incomplete at {}", e.name, p.display());
                return 1;
            }
            let errors = cfg.validation_errors();
            if errors.is_empty() {
                println!("active {} model is ready", cfg.engine.backend);
                return 0;
            }
            for error in errors {
                eprintln!("{error}");
            }
            return 1;
        }
        "install" => args
            .get(3)
            .ok_or_else(|| "usage: dictation model install <name>".to_string())
            .and_then(|n| {
                known(n)
                    .ok_or_else(|| format!("unknown model `{n}`; run dictation model list"))
                    .and_then(|e| {
                        let p = model_path(cfg, e);
                        download(e, &p)?;
                        println!("installed {} at {}", e.name, p.display());
                        println!("To make it your default model, run: dictation model set {}", e.name);
                        Ok(())
                    })
            }),
        "set" => args
            .get(3)
            .ok_or_else(|| "usage: dictation model set <name|path>".to_string())
            .and_then(|value| {
                let mut next = cfg.clone();
                if let Some(e) = known(value) {
                    let p = model_path(cfg, e);
                    if !valid(e, &p) {
                        return Err(format!(
                            "{} is not installed; run dictation model install {}",
                            e.name, e.name
                        ));
                    }
                    next.engine.backend = e.backend.into();
                    match e.backend {
                        "whisper" => next.engine.whisper_model = p.to_string_lossy().into_owned(),
                        "moonshine" => {
                            next.engine.moonshine_model_dir = p.to_string_lossy().into_owned()
                        }
                        "zipformer" => {
                            next.engine.zipformer_model_dir = p.to_string_lossy().into_owned()
                        }
                        "vosk" => {
                            next.engine.backend = "vosk".into();
                            next.engine.model_path = p.to_string_lossy().into_owned();
                        }
                        "sensevoice" => {
                            next.engine.sense_voice_model_dir = p.to_string_lossy().into_owned()
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let p = PathBuf::from(value);
                    if !p.is_file() {
                        return Err(format!("model file not found: {}", p.display()));
                    }
                    next.engine.backend = "whisper".into();
                    next.engine.whisper_model = p.to_string_lossy().into_owned();
                }
                save(&next)?;
                let selected_path = match next.engine.backend.as_str() {
                    "whisper" => &next.engine.whisper_model,
                    "vosk" | "native" => &next.engine.model_path,
                    "moonshine" => &next.engine.moonshine_model_dir,
                    "zipformer" => &next.engine.zipformer_model_dir,
                    "sensevoice" => &next.engine.sense_voice_model_dir,
                    _ => unreachable!(),
                };
                println!("Default model saved: {} ({})", selected_path, next.engine.backend);
                println!("It will be used the next time dictation starts; stop any current session first.");
                Ok(())
            }),
        "remove" => args
            .get(3)
            .ok_or_else(|| "usage: dictation model remove <name>".to_string())
            .and_then(|n| {
                known(n)
                    .ok_or_else(|| format!("unknown model `{n}`"))
                    .and_then(|e| {
                        let p = model_path(cfg, e);
                        if !p.starts_with(root()) {
                            return Err(
                                "refusing to remove a path outside the managed model directory"
                                    .into(),
                            );
                        }
                        if cfg.engine.backend == e.backend
                            && match e.backend {
                                "whisper" => Path::new(&cfg.engine.whisper_model) == p,
                                "moonshine" => Path::new(&cfg.engine.moonshine_model_dir) == p,
                                "zipformer" => Path::new(&cfg.engine.zipformer_model_dir) == p,
                                "vosk" => Path::new(&cfg.engine.model_path) == p,
                                "sensevoice" => Path::new(&cfg.engine.sense_voice_model_dir) == p,
                                _ => false,
                            }
                        {
                            return Err(
                                "cannot remove the active model; set another model first".into()
                            );
                        }
                        if !p.exists() {
                            return Err(format!("{} is not installed", e.name));
                        }
                        if p.is_dir() {
                            std::fs::remove_dir_all(&p)
                        } else {
                            std::fs::remove_file(&p)
                        }
                        .map_err(|x| x.to_string())?;
                        println!("removed {}", e.name);
                        Ok(())
                    })
            }),
        "recommend" => {
            let mib = std::fs::read_to_string("/proc/meminfo")
                .ok()
                .and_then(|s| {
                    s.lines()
                        .find(|l| l.starts_with("MemAvailable:"))
                        .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
                })
                .unwrap_or(0)
                / 1024;
            let choice = if mib > 6000 {
                "whisper-small.en"
            } else if mib > 2500 {
                "whisper-base.en"
            } else {
                "whisper-tiny.en"
            };
            println!(
                "Recommended starting point: {choice} (about {mib} MiB memory currently available). This is a rough memory-based suggestion; benchmark on your device for speed and recognition quality."
            );
            return 0;
        }
        "benchmark" => {
            let mut forwarded = vec!["dictation".to_string(), "benchmark".to_string()];
            for arg in args.iter().skip(3) {
                if let Some(e) = known(arg) {
                    forwarded.push("--model".into());
                    forwarded.push(model_path(cfg, e).to_string_lossy().into_owned());
                } else {
                    forwarded.push(arg.clone());
                }
            }
            return match super::benchmark::run(&forwarded, cfg) {
                Ok(report) => {
                    print!("{report}");
                    0
                }
                Err(e) => {
                    eprintln!("dictation model benchmark: {e}");
                    1
                }
            };
        }
        other => Err(format!(
            "unknown model command `{other}`; use check, list, install, set, remove, recommend, benchmark, management"
        )),
    };
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("dictation model: {e}");
            1
        }
    }
}
