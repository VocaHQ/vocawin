//! Voca Hinglish output filter, matching VocaMac's
//! `WhisperService.removingUnexpectedScripts`.
//!
//! The model writes Latin letters, and now and then Devanagari. When it
//! derails it can write another script entirely: one dictation ended in
//! "в ктттт…". Letters from any other script are decoder garbage and are
//! removed; a word keeps whatever it held besides them ("amazing.в" stays
//! "amazing."), and a word left without letters or digits is dropped. A
//! take that is only `nan` is silence (see `clean`).

/// Voca Hinglish's text for a take. It writes `nan` when it hears only
/// silence or noise (its training data labelled silent clips that way);
/// that take has no text. Anything else keeps only Latin and Devanagari.
pub fn clean(text: &str) -> String {
    // Filtered first: "nan в" is still silence once the stray word goes.
    let text = keep_expected_scripts(text);
    let words = text.trim().trim_matches(|ch: char| ch.is_ascii_punctuation());
    if words.eq_ignore_ascii_case("nan") {
        return String::new();
    }
    text
}

/// `text` with only Latin and Devanagari letters. Line breaks and the
/// spacing between kept words stay as they were; only the gap a removed
/// word leaves is closed up.
pub fn keep_expected_scripts(text: &str) -> String {
    if !text.chars().any(off_script) {
        return text.to_string();
    }
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        let word_start = rest.find(|ch: char| !ch.is_whitespace()).unwrap_or(rest.len());
        result.push_str(&rest[..word_start]);
        rest = &rest[word_start..];
        let word_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let word = &rest[..word_end];
        rest = &rest[word_end..];
        if !word.chars().any(off_script) {
            result.push_str(word);
            continue;
        }
        let mut kept = String::with_capacity(word.len());
        let mut dropping = false;
        for ch in word.chars() {
            if off_script(ch) {
                dropping = true;
            } else if dropping && is_mark(ch) {
                // A mark attached to a removed letter goes with it.
            } else {
                dropping = false;
                kept.push(ch);
            }
        }
        if kept.chars().any(char::is_alphanumeric) {
            result.push_str(&kept);
        }
    }
    close_gaps(&result)
}

/// Runs of spaces and tabs become one space, and the ends are trimmed, as
/// VocaMac does after removing words. Line breaks are kept.
fn close_gaps(text: &str) -> String {
    let mut closed = String::with_capacity(text.len());
    let mut in_gap = false;
    for ch in text.chars() {
        if ch == ' ' || ch == '\t' {
            if !in_gap {
                closed.push(' ');
            }
            in_gap = true;
        } else {
            closed.push(ch);
            in_gap = false;
        }
    }
    closed.trim().to_string()
}

/// A letter outside Latin and Devanagari.
fn off_script(ch: char) -> bool {
    ch.is_alphabetic() && !is_latin(ch) && !is_devanagari(ch)
}

fn is_latin(ch: char) -> bool {
    ch.is_ascii_alphabetic()
        || matches!(ch as u32,
            0x00AA | 0x00BA
            | 0x00C0..=0x00D6
            | 0x00D8..=0x00F6
            | 0x00F8..=0x024F
            | 0x1E00..=0x1EFF
            | 0x2C60..=0x2C7F
            | 0xA720..=0xA7FF
            | 0xFF21..=0xFF3A
            | 0xFF41..=0xFF5A)
}

fn is_devanagari(ch: char) -> bool {
    matches!(ch as u32, 0x0900..=0x097F | 0xA8E0..=0xA8FF | 0x1CD0..=0x1CFF)
}

/// A combining mark that is not itself a letter (Rust has no general
/// category lookup; these blocks cover the scripts Whisper derails into).
fn is_mark(ch: char) -> bool {
    matches!(ch as u32,
        0x0300..=0x036F
        | 0x0483..=0x0489
        | 0x0591..=0x05C7
        | 0x0610..=0x061A
        | 0x064B..=0x065F
        | 0x0670
        | 0x06D6..=0x06ED
        | 0x0E31..=0x0E3A
        | 0x0E47..=0x0E4E
        | 0x1AB0..=0x1AFF
        | 0x1DC0..=0x1DFF
        | 0x20D0..=0x20FF
        | 0x3099..=0x309A
        | 0xFE20..=0xFE2F)
}

#[cfg(test)]
mod tests {
    use super::{clean, keep_expected_scripts};

    #[test]
    fn nan_for_silence_is_no_text() {
        for text in ["nan", " nan", "Nan.", "NaN"] {
            assert_eq!(clean(text), "", "{text:?}");
        }
        assert_eq!(clean("Naan aur daal chahiye."), "Naan aur daal chahiye.");
        assert_eq!(clean("nan bhai"), "nan bhai");
        assert_eq!(clean("nan в"), "");
    }

    #[test]
    fn filtering_keeps_line_breaks() {
        assert_eq!(
            keep_expected_scripts("Pehli line hai.\nDoosri в line."),
            "Pehli line hai.\nDoosri line."
        );
    }

    #[test]
    fn words_in_other_scripts_are_dropped() {
        assert_eq!(
            keep_expected_scripts("Cats are everywhere. It is just amazing. в кт"),
            "Cats are everywhere. It is just amazing."
        );
        assert_eq!(
            keep_expected_scripts("Haan 谢谢 thik hai, مرحبا bhai."),
            "Haan thik hai, bhai."
        );
    }

    #[test]
    fn the_latin_part_of_a_word_with_garbage_attached_is_kept() {
        assert_eq!(keep_expected_scripts("It is just amazing.в кт"), "It is just amazing.");
        assert_eq!(keep_expected_scripts("Приветbhai, kaise ho? «в»"), "bhai, kaise ho?");
        assert_eq!(keep_expected_scripts("Room 404в mein"), "Room 404 mein");
    }

    #[test]
    fn latin_and_devanagari_are_kept_untouched() {
        for text in [
            "Aap pandrah log hain.",
            "हाँ ठीक है, kal milte hain.",
            "Café mein naïve sa sawaal, 1,500 rupaye!",
            "Deadline 5:30 pm hai 🙂",
            "",
        ] {
            assert_eq!(keep_expected_scripts(text), text, "{text:?}");
        }
    }
}
