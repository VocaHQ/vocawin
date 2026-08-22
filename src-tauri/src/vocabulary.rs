//! Custom Vocabulary, matching VocaPhone `CustomVocabulary` (Android + iOS).
//!
//! The stored list is a Win setting, not a shared family config key. Whisper.cpp
//! has no vocabulary parameter; the engine field is `initial_prompt`.
//! Split on newlines and commas, not spaces. Case-insensitive dedup keeps the
//! first spelling. Term max 64, prompt max 640, join with ", " and a trailing
//! period.

const MAX_PROMPT_CHARACTERS: usize = 640;
const MAX_TERM_CHARACTERS: usize = 64;

/// Distinct terms in the order the user wrote them.
pub fn terms(raw: &str) -> Vec<String> {
    if raw.trim().is_empty() {
        return Vec::new();
    }
    let mut seen = std::collections::HashSet::new();
    raw.split(['\n', ','])
        .map(|part| take_chars(part.trim(), MAX_TERM_CHARACTERS).trim().to_string())
        .filter(|part| !part.is_empty())
        .filter(|part| seen.insert(part.to_lowercase()))
        .collect()
}

/// `initial_prompt` for whisper.cpp. Empty when there are no usable terms.
pub fn whisper_prompt(raw: &str) -> String {
    let terms = terms(raw);
    if terms.is_empty() {
        return String::new();
    }
    let mut prompt = String::new();
    for term in terms {
        let separator = if prompt.is_empty() { "" } else { ", " };
        if prompt.len() + separator.len() + term.len() > MAX_PROMPT_CHARACTERS {
            break;
        }
        prompt.push_str(separator);
        prompt.push_str(&term);
    }
    if prompt.is_empty() {
        String::new()
    } else {
        format!("{prompt}.")
    }
}

fn take_chars(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_split_on_newlines_and_commas_but_never_inside_a_phrase() {
        assert_eq!(
            terms("Claude Code\nTailscale, VocaPhone"),
            vec!["Claude Code", "Tailscale", "VocaPhone"]
        );
    }

    #[test]
    fn first_spelling_of_a_duplicate_is_kept() {
        assert_eq!(
            terms("VocaPhone, vocaphone, VOCAPHONE"),
            vec!["VocaPhone"]
        );
    }

    #[test]
    fn blank_entries_and_stray_separators_are_dropped() {
        assert!(terms("").is_empty());
        assert!(terms("  \n , , \n ").is_empty());
        assert_eq!(terms(",\n Kanishk ,\n"), vec!["Kanishk"]);
    }

    #[test]
    fn prompt_is_comma_separated_with_a_trailing_period() {
        assert_eq!(
            whisper_prompt("Kanishk\nVocaHQ\nTailscale"),
            "Kanishk, VocaHQ, Tailscale."
        );
        assert_eq!(whisper_prompt(""), "");
    }

    #[test]
    fn over_long_list_truncates_at_a_term_boundary() {
        let raw = (1..=200)
            .map(|n| format!("Supercalifragilistic{n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = whisper_prompt(&raw);
        assert!(prompt.len() <= 641, "prompt should be bounded");
        let body = prompt.strip_suffix('.').unwrap_or(&prompt);
        for term in body.split(", ") {
            assert!(
                term.starts_with("Supercalifragilistic"),
                "`{term}` should be whole"
            );
        }
    }
}
