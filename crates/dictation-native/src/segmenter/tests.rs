#[cfg(test)]
mod tests {
    use super::whisper::strip_parens;

    #[test]
    fn stage_directions_are_dropped_but_speech_is_not() {
        // Placeholder-only: noise that got past the gates types nothing.
        assert_eq!(strip_parens("(upbeat music)"), "");
        assert_eq!(strip_parens("(crickets chirping)"), "");
        // Embedded annotations vanish; the words around them survive.
        assert_eq!(strip_parens("hello (laughs) world"), "hello world");
        assert_eq!(strip_parens("hello (laughs)"), "hello");
        // Plain speech passes through untouched.
        assert_eq!(strip_parens("no parens here"), "no parens here");
        // Unbalanced: refuse to guess rather than eat the rest of the line.
        assert_eq!(strip_parens("unclosed (laughs"), "unclosed (laughs");
    }
}
