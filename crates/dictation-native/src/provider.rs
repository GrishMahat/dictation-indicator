//! One-shot compute-provider preflight.
//!
//! The app chooses a candidate once at startup — never per frame and never by
//! watching load. This checks build features and visible devices; Whisper.cpp
//! can still fall back to CPU if GPU initialization fails later.

use dictation_config::{EngineConfig, Provider};

/// Device that `auto` or a provider pin asks Whisper.cpp to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderCandidate {
    Cpu,
    Vulkan,
    Cuda,
    Hip,
    Metal,
}

impl ProviderCandidate {
    /// Spelling used in logs and diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Vulkan => "vulkan",
            Self::Cuda => "cuda",
            Self::Hip => "hip",
            Self::Metal => "metal",
        }
    }

    /// Whether recognition should hand work to a GPU backend.
    pub fn is_gpu(self) -> bool {
        self != Self::Cpu
    }
}

/// Cargo feature that must be enabled for a pinned provider.
const REBUILD_HINT: &[(Provider, &str)] = &[
    (Provider::Vulkan, "gpu-vulkan"),
    (Provider::Cuda, "gpu-cuda"),
    (Provider::Hip, "gpu-hip"),
    (Provider::Metal, "gpu-metal"),
];

/// GPU features compiled into this binary, for diagnostics (`"none"` = none).
pub fn compiled_features() -> String {
    let joined = [
        (cfg!(feature = "gpu-cuda"), "gpu-cuda"),
        (cfg!(feature = "gpu-hip"), "gpu-hip"),
        (cfg!(feature = "gpu-vulkan"), "gpu-vulkan"),
        (cfg!(feature = "gpu-metal"), "gpu-metal"),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(_, name)| name)
    .collect::<Vec<_>>()
    .join(", ");
    if joined.is_empty() {
        "none".to_owned()
    } else {
        joined
    }
}

/// Choose a provider candidate for this engine config.
///
/// This is a preflight decision, not proof that Whisper.cpp successfully
/// initialized the GPU backend. Missing build support or visible hardware is
/// reported before model loading.
pub fn resolve(engine: &EngineConfig) -> Result<ProviderCandidate, String> {
    // Only the Whisper backend ships GPU code paths: the sherpa-onnx
    // prebuilt runtime and Vosk are CPU-only regardless of config.
    if engine.backend != "whisper" {
        return if engine.provider.is_gpu() {
            Err(format!(
                "provider `{}` needs the whisper backend; backend `{}` runs on CPU only",
                engine.provider.as_str(),
                engine.backend
            ))
        } else {
            Ok(ProviderCandidate::Cpu)
        };
    }
    match engine.provider {
        Provider::Cpu => Ok(ProviderCandidate::Cpu),
        Provider::Auto | Provider::Vulkan | Provider::Cuda | Provider::Hip | Provider::Metal => {
            let providers = compiled_gpu_providers();
            if providers.len() > 1 {
                let names = providers
                    .iter()
                    .map(|provider| provider.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!(
                    "this build includes multiple Whisper GPU providers ({names}), but whisper.cpp does not let this app pin a provider; rebuild with one GPU feature"
                ));
            }
            if engine.provider == Provider::Auto {
                Ok(probe_auto())
            } else {
                pin(engine.provider)
            }
        }
    }
}

fn compiled_gpu_providers() -> Vec<Provider> {
    [
        Provider::Cuda,
        Provider::Hip,
        Provider::Vulkan,
        Provider::Metal,
    ]
    .into_iter()
    .filter(|provider| compiled(*provider))
    .collect()
}

/// Probe the single compiled-in GPU backend and return it if a device is
/// visible. Runs once at startup.
fn probe_auto() -> ProviderCandidate {
    // The order is also used by compiled_features and the multi-feature error.
    [
        (Provider::Cuda, ProviderCandidate::Cuda),
        (Provider::Hip, ProviderCandidate::Hip),
        (Provider::Vulkan, ProviderCandidate::Vulkan),
        (Provider::Metal, ProviderCandidate::Metal),
    ]
    .into_iter()
    .find(|(configured, _)| compiled(*configured) && device_present(*configured))
    .map_or(ProviderCandidate::Cpu, |(_, candidate)| candidate)
}

/// A pinned provider must be compiled in and have a visible device.
fn pin(pinned: Provider) -> Result<ProviderCandidate, String> {
    let (_, feature) = REBUILD_HINT
        .iter()
        .find(|(provider, _)| *provider == pinned)
        .expect("every GPU provider lists a rebuild hint");
    if !compiled(pinned) {
        return Err(format!(
            "provider `{}` is not compiled into this build; \
             rebuild with `--features {feature}`",
            pinned.as_str()
        ));
    }
    if !device_present(pinned) {
        return Err(format!(
            "provider `{}` was requested but no {} device was found",
            pinned.as_str(),
            pinned.as_str()
        ));
    }
    Ok(match pinned {
        Provider::Vulkan => ProviderCandidate::Vulkan,
        Provider::Cuda => ProviderCandidate::Cuda,
        Provider::Hip => ProviderCandidate::Hip,
        Provider::Metal => ProviderCandidate::Metal,
        Provider::Auto | Provider::Cpu => unreachable!("pinned providers are GPUs"),
    })
}

/// Whether this build contains the backend at all (compile-time).
fn compiled(provider: Provider) -> bool {
    match provider {
        Provider::Vulkan => cfg!(feature = "gpu-vulkan"),
        Provider::Cuda => cfg!(feature = "gpu-cuda"),
        Provider::Hip => cfg!(feature = "gpu-hip"),
        Provider::Metal => cfg!(feature = "gpu-metal"),
        Provider::Auto | Provider::Cpu => false,
    }
}

/// Whether a device for this backend is visible right now (runtime).
fn device_present(provider: Provider) -> bool {
    match provider {
        Provider::Vulkan => vulkan_device_present(),
        Provider::Cuda => dev_dir_has_numbered_device(std::path::Path::new("/dev"), "nvidia"),
        Provider::Hip => {
            std::path::Path::new("/dev/kfd").exists()
                && dev_dir_has_numbered_device(std::path::Path::new("/dev/dri"), "renderD")
        }
        // Metal only exists inside Apple targets.
        Provider::Metal => cfg!(target_os = "macos"),
        Provider::Auto | Provider::Cpu => true,
    }
}

/// Real Vulkan enumeration through ggml — accurate, and free at startup.
#[cfg(feature = "gpu-vulkan")]
fn vulkan_device_present() -> bool {
    !whisper_rs::vulkan::list_devices().is_empty()
}

#[cfg(not(feature = "gpu-vulkan"))]
fn vulkan_device_present() -> bool {
    false
}

/// True when a device directory contains `prefix` followed by a number.
/// This avoids treating control nodes such as `nvidia-modeset` as GPUs.
fn dev_dir_has_numbered_device(directory: &std::path::Path, prefix: &str) -> bool {
    std::fs::read_dir(directory).is_ok_and(|entries| {
        entries.filter_map(Result::ok).any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .strip_prefix(prefix)
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
                })
        })
    })
}

/// Threads for this engine: an explicit `engine.threads`, or the backend's
/// tuned default when set to `0` (auto).
pub(crate) fn resolve_threads(engine: &EngineConfig, auto: i32) -> i32 {
    match i32::try_from(engine.threads) {
        // Validation caps this at 128; clamp anyway so an unchecked config
        // cannot wrap into a negative thread count.
        Ok(threads) if threads > 0 => threads.min(128),
        _ => auto,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> EngineConfig {
        EngineConfig::default()
    }

    /// `auto` and `cpu` must resolve on every backend, GPU pin or not.
    #[test]
    fn non_whisper_backends_stay_on_cpu() {
        for backend in ["vosk", "moonshine", "zipformer", "sensevoice"] {
            for provider in [Provider::Auto, Provider::Cpu] {
                let mut cfg = engine();
                cfg.backend = backend.into();
                cfg.provider = provider;
                assert_eq!(
                    resolve(&cfg),
                    Ok(ProviderCandidate::Cpu),
                    "{backend}/{provider:?}",
                );
            }
        }
    }

    /// Config validation catches this too; resolve() stays defensive anyway.
    #[test]
    fn gpu_pin_on_cpu_only_backend_fails_loudly() {
        let mut cfg = engine();
        cfg.backend = "zipformer".into();
        cfg.provider = Provider::Cuda;
        let error = resolve(&cfg).expect_err("must not silently ignore a pin");
        assert!(error.contains("whisper backend"), "got: {error}");
        assert!(error.contains("CPU only"), "got: {error}");
    }

    #[test]
    fn cpu_pin_always_resolves_to_cpu() {
        let mut cfg = engine();
        cfg.provider = Provider::Cpu;
        assert_eq!(resolve(&cfg), Ok(ProviderCandidate::Cpu));
    }

    /// `auto` resolves with zero or one GPU feature, and rejects ambiguous
    /// binaries rather than reporting a provider Whisper may not choose.
    #[test]
    fn auto_rejects_ambiguous_gpu_builds() {
        let providers = compiled_gpu_providers();
        let result = resolve(&engine());
        if providers.len() > 1 {
            assert!(
                result.is_err_and(|error| error.contains("multiple Whisper GPU providers")),
                "ambiguous provider build should explain the conflict"
            );
        } else if providers.is_empty() {
            assert_eq!(result, Ok(ProviderCandidate::Cpu));
        } else {
            assert!(result.is_ok(), "single-provider build should resolve");
        }
    }

    /// A pin with its feature compiled out is a build error, not a fallback.
    #[test]
    #[cfg(not(feature = "gpu-vulkan"))]
    fn pinning_an_uncompiled_provider_explains_the_rebuild() {
        let mut cfg = engine();
        cfg.provider = Provider::Vulkan;
        let error = resolve(&cfg).expect_err("uncompiled pin must fail");
        if compiled_gpu_providers().len() > 1 {
            assert!(
                error.contains("multiple Whisper GPU providers"),
                "got: {error}"
            );
        } else {
            assert!(error.contains("--features gpu-vulkan"), "got: {error}");
        }
    }

    #[test]
    fn threads_zero_means_auto_and_the_cap_holds() {
        let mut cfg = engine();
        assert_eq!(resolve_threads(&cfg, 8), 8);
        cfg.threads = 4;
        assert_eq!(resolve_threads(&cfg, 8), 4);
        cfg.threads = 512;
        assert_eq!(resolve_threads(&cfg, 8), 128, "clamp before the i32 cast");
    }

    #[test]
    fn compiled_features_reports_none_by_default() {
        let features = compiled_features();
        if cfg!(any(
            feature = "gpu-vulkan",
            feature = "gpu-cuda",
            feature = "gpu-hip",
            feature = "gpu-metal"
        )) {
            assert_ne!(features, "none");
        } else {
            assert_eq!(features, "none");
        }
    }
}
