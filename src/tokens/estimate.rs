//! Character-based token estimate (ceil(chars / 4)) for models without
//! an exact tokenizer.

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
        assert_eq!(count("a"), 1);
        assert_eq!(count("ab"), 1);
        assert_eq!(count("abcd"), 1);
        assert_eq!(count("abcde"), 2);
    }

    #[test]
    fn multibyte_counts_characters_not_bytes() {
        assert_eq!(count("café"), 1);
    }
}
