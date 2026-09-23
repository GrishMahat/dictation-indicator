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
