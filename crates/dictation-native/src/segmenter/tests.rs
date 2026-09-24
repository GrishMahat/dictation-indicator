// Loaded by `segmenter/mod.rs` as `mod tests` (already `#[cfg(test)]` there),
// so items here live directly in `segmenter::tests` — no inner wrapper.
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

/// `buf_ms` must return milliseconds. With `* 100` it returned
/// deciseconds while every consumer (`FIRST_CHUNK_MS`, `MIN_SPEECH_MS`,
/// `SILENCE_END_MS`, `MAX_SEGMENT_MS`, and the `_ms` accumulators)
/// treated it as ms — so each threshold took 10× its wall-time:
/// first text at 10 s, pauses needing 5 s to end a segment, 60 s
/// splits, and sub-2 s utterances failing the `heard` check in
/// `finalize` and being silently dropped.
#[test]
fn buf_ms_reports_milliseconds_not_deciseconds() {
    use super::whisper::buf_ms;
    let rate = crate::audio::RATE as usize;
    assert_eq!(buf_ms(rate), 1000); // 1 s of audio = 1000 ms
    assert_eq!(buf_ms(rate / 10), 100); // one 100 ms capture chunk
    assert_eq!(buf_ms(rate / 100), 10); // 10 ms
    assert_eq!(buf_ms(rate * 6), 6000); // MAX_SEGMENT_MS window
}
