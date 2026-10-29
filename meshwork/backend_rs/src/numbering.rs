//! Human-readable task numbers. Every graph has a short `key` (`MW`) and a counter
//! (`Graph.task_seq`); each new task takes the next number, so it can be referred to as
//! `MW-12` in branches, commits and task routes. Numbers are never reused.

/// Longest graph key accepted.
pub const MAX_KEY_LEN: usize = 10;

/// Key used when a graph's name gives nothing to derive one from (e.g. a non-Latin name).
pub const FALLBACK_KEY: &str = "T";

/// Words skipped when a key is derived from initials.
const STOP_WORDS: [&str; 10] = [
    "a", "an", "the", "of", "and", "to", "for", "in", "on", "with",
];

/// The key a graph gets from its name unless one is given: the initials of its words (up to
/// four, stop words skipped), or the first three letters of a single word, uppercase ASCII.
/// [`FALLBACK_KEY`] when the name has no Latin words.
pub fn default_key(name: &str) -> String {
    let words: Vec<&str> = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.starts_with(|c: char| c.is_ascii_alphabetic()))
        .collect();
    let significant: Vec<&str> = words
        .iter()
        .copied()
        .filter(|w| !STOP_WORDS.contains(&w.to_ascii_lowercase().as_str()))
        .collect();
    let words = if significant.is_empty() {
        words
    } else {
        significant
    };
    let key: String = match words.as_slice() {
        [] => return FALLBACK_KEY.to_string(),
        [word] => word.chars().take(3).collect(),
        _ => words
            .iter()
            .take(4)
            .filter_map(|w| w.chars().next())
            .collect(),
    };
    key.to_ascii_uppercase()
}

/// A user-given key, trimmed and uppercased, if it is valid: 1 to [`MAX_KEY_LEN`] ASCII
/// letters and digits, starting with a letter.
pub fn normalize_key(key: &str) -> Option<String> {
    let key = key.trim().to_ascii_uppercase();
    let valid = !key.is_empty()
        && key.len() <= MAX_KEY_LEN
        && key.starts_with(|c: char| c.is_ascii_alphabetic())
        && key.chars().all(|c| c.is_ascii_alphanumeric());
    valid.then_some(key)
}

/// `MW-12`.
pub fn format_key(graph_key: &str, number: i64) -> String {
    format!("{}-{}", graph_key, number)
}

/// How a task route names a task.
#[derive(Debug, PartialEq, Eq)]
pub enum TaskRef<'a> {
    /// The task's ID (UUID), or anything that is not a number.
    Id(&'a str),
    /// `12` or `MW-12` (any case); `key` is uppercased.
    Number { key: Option<String>, number: i64 },
}

pub fn parse_ref(s: &str) -> TaskRef<'_> {
    fn number(digits: &str) -> Option<i64> {
        if digits.is_empty() || digits.len() > 18 || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        digits.parse().ok()
    }
    if let Some(n) = number(s) {
        return TaskRef::Number {
            key: None,
            number: n,
        };
    }
    if let Some((prefix, digits)) = s.rsplit_once('-') {
        if let (Some(key), Some(n)) = (normalize_key(prefix), number(digits)) {
            // normalize_key trims; a prefix with spaces is not a key.
            if key.len() == prefix.len() {
                return TaskRef::Number {
                    key: Some(key),
                    number: n,
                };
            }
        }
    }
    TaskRef::Id(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_keys_from_names() {
        assert_eq!(default_key("Finish the Meshwork MVP"), "FMM");
        assert_eq!(default_key("Meshwork"), "MES");
        assert_eq!(default_key("Ship v2 of the app"), "SVA");
        assert_eq!(default_key("one two three four five"), "OTTF");
        assert_eq!(default_key("The"), "THE");
        assert_eq!(default_key(""), FALLBACK_KEY);
        assert_eq!(default_key("Запустить сайт"), FALLBACK_KEY);
        assert_eq!(default_key("42"), FALLBACK_KEY);
    }

    #[test]
    fn normalizes_keys() {
        assert_eq!(normalize_key(" mw "), Some("MW".to_string()));
        assert_eq!(normalize_key("A1"), Some("A1".to_string()));
        assert_eq!(normalize_key("1A"), None);
        assert_eq!(normalize_key("M-W"), None);
        assert_eq!(normalize_key(""), None);
        assert_eq!(normalize_key("ABCDEFGHIJK"), None);
    }

    #[test]
    fn parses_task_refs() {
        assert_eq!(
            parse_ref("12"),
            TaskRef::Number {
                key: None,
                number: 12
            }
        );
        assert_eq!(
            parse_ref("mw-7"),
            TaskRef::Number {
                key: Some("MW".to_string()),
                number: 7
            }
        );
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(parse_ref(uuid), TaskRef::Id(uuid));
        assert_eq!(parse_ref("MW-"), TaskRef::Id("MW-"));
        assert_eq!(parse_ref("MW-1x"), TaskRef::Id("MW-1x"));
        assert_eq!(parse_ref("t1"), TaskRef::Id("t1"));
    }
}
