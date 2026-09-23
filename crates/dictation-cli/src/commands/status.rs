use dictation_config::Config;
use dictation_core::{SessionState, session_state};

pub(super) fn run(config: &Config) -> i32 {
    match session_state() {
        SessionState::Listening => {
            println!("on — listening (backend: {})", config.engine.backend)
        }
        SessionState::Dictating => {
            println!(
                "finishing — dictating final audio (backend: {})",
                config.engine.backend
            )
        }
        SessionState::Off => println!("off"),
    }
    0
}
