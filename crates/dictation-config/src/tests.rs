use super::*;

/// Every supported typing backend spelling must round-trip through TOML.
#[test]
fn typing_backend_spellings_round_trip() {
    let all = [
        (TypingBackend::Uinput, "uinput"),
        (TypingBackend::Wtype, "wtype"),
        (TypingBackend::Dotool, "dotool"),
        (TypingBackend::Ydotool, "ydotool"),
        (TypingBackend::Xdotool, "xdotool"),
    ];
    for (backend, spelling) in all {
        assert_eq!(backend.as_str(), spelling);
        let toml = format!("typing_backend = \"{spelling}\"\n");
        let parsed: EngineConfig = toml::from_str(&toml).expect("parses");
        assert_eq!(parsed.typing_backend, backend);
        assert_eq!(parsed.backend, EngineConfig::default().backend);
    }
}

#[test]
fn typing_backend_defaults_to_uinput() {
    assert_eq!(TypingBackend::default(), TypingBackend::Uinput);
    assert_eq!(
        EngineConfig::default().typing_backend,
        TypingBackend::Uinput
    );
    assert!(default_toml().contains("typing_backend = \"uinput\""));
}

/// Every supported provider spelling must round-trip through TOML.
#[test]
fn provider_spellings_round_trip() {
    let all = [
        (Provider::Auto, "auto"),
        (Provider::Cpu, "cpu"),
        (Provider::Vulkan, "vulkan"),
        (Provider::Cuda, "cuda"),
        (Provider::Hip, "hip"),
        (Provider::Metal, "metal"),
    ];
    for (provider, spelling) in all {
        assert_eq!(provider.as_str(), spelling);
        let toml = format!("provider = \"{spelling}\"\n");
        let parsed: EngineConfig = toml::from_str(&toml).expect("parses");
        assert_eq!(parsed.provider, provider);
        assert_eq!(parsed.backend, EngineConfig::default().backend);
    }
}

#[test]
fn provider_defaults_to_auto_and_rejects_typos() {
    assert_eq!(Provider::default(), Provider::Auto);
    assert_eq!(EngineConfig::default().provider, Provider::Auto);
    assert!(default_toml().contains("provider = \"auto\""));
    assert!(toml::from_str::<EngineConfig>("provider = \"cuda \"\n").is_err());
    assert!(toml::from_str::<EngineConfig>("provider = \"rocm\"\n").is_err());
    assert!(Provider::Cuda.is_gpu());
    assert!(Provider::Vulkan.is_gpu());
    assert!(!Provider::Auto.is_gpu());
    assert!(!Provider::Cpu.is_gpu());
}

/// A pinned GPU provider on a CPU-only backend would be silently ignored,
/// so validation must reject it while leaving `auto`/`cpu` alone.
#[test]
fn gpu_provider_requires_the_whisper_backend() {
    let message = "engine.provider = \"cuda\" is only supported by the whisper backend";
    for backend in ["vosk", "moonshine", "zipformer", "sensevoice"] {
        let cfg = EngineConfig {
            backend: backend.into(),
            provider: Provider::Cuda,
            ..EngineConfig::default()
        };
        assert!(
            cfg_provider_error(&cfg).is_some_and(|e| e.contains(message)),
            "backend `{backend}` must reject a pinned GPU provider"
        );
    }
    for provider in [Provider::Auto, Provider::Cpu] {
        let cfg = EngineConfig {
            backend: "moonshine".into(),
            provider,
            ..EngineConfig::default()
        };
        assert!(
            cfg_provider_error(&cfg).is_none(),
            "provider `{}` must be allowed on CPU-only backends",
            provider.as_str()
        );
    }
    let cfg = EngineConfig {
        provider: Provider::Vulkan,
        ..EngineConfig::default()
    };
    assert!(
        cfg_provider_error(&cfg).is_none(),
        "whisper must accept a pinned GPU provider"
    );
}

/// `threads = 0` means auto; anything past the oversubscription cap is an error.
#[test]
fn thread_counts_are_auto_or_bounded() {
    assert_eq!(EngineConfig::default().threads, 0);
    assert!(default_toml().contains("threads = 0"));
    let parsed: EngineConfig = toml::from_str("threads = 8\n").expect("parses");
    assert_eq!(parsed.threads, 8);
    let over_cap = EngineConfig {
        threads: 129,
        ..EngineConfig::default()
    };
    assert!(
        config_with_engine(&over_cap)
            .validation_errors()
            .iter()
            .any(|e| e.contains("engine.threads")),
    );
    let at_cap = EngineConfig {
        threads: 128,
        ..EngineConfig::default()
    };
    assert!(
        !config_with_engine(&at_cap)
            .validation_errors()
            .iter()
            .any(|e| e.contains("engine.threads")),
    );
    let vosk_threads = EngineConfig {
        backend: "vosk".into(),
        threads: 4,
        ..EngineConfig::default()
    };
    assert!(
        config_with_engine(&vosk_threads)
            .validation_errors()
            .iter()
            .any(|e| e.contains("engine.threads is not supported by Vosk"))
    );
    // Negative counts never reach validation — serde rejects them outright.
    assert!(toml::from_str::<EngineConfig>("threads = -1\n").is_err());
}

/// Validation errors mentioning `engine.provider` for a config with these engine settings.
fn cfg_provider_error(engine: &EngineConfig) -> Option<String> {
    config_with_engine(engine)
        .validation_errors()
        .into_iter()
        .find(|e| e.contains("engine.provider"))
}

/// Config carrying only the given engine settings; indicator defaults elsewhere.
fn config_with_engine(engine: &EngineConfig) -> Config {
    Config {
        engine: engine.clone(),
        ..Config::default()
    }
}
