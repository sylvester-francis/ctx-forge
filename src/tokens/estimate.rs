//! Character-based token estimate. Used for any model without an exact
//! tokenizer. Matches the "chars / 4" rule-of-thumb that Anthropic and
//! OpenAI both cite as a rough approximation for English-ish text.

/// Estimate tokens as ceil(chars / 4).
pub fn count(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_zero() {
        assert_eq!(count(""), 0);
    }

    #[test]
    fn short_string_rounds_up() {
        assert_eq!(count("a"), 1); // 1 char → ceil(1/4) = 1
        assert_eq!(count("ab"), 1);
        assert_eq!(count("abcd"), 1);
        assert_eq!(count("abcde"), 2); // 5 chars → ceil(5/4) = 2
    }

    #[test]
    fn multibyte_counts_characters_not_bytes() {
        // "café" has 4 chars but 5 bytes. Token estimate should use chars.
        assert_eq!(count("café"), 1);
    }
}
