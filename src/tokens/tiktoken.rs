//! Exact token counting via the `tiktoken-rs` crate.

use tiktoken_rs::{cl100k_base, o200k_base};

pub fn count(text: &str, encoding: &str) -> usize {
    match encoding {
        "cl100k_base" => cl100k_base()
            .map(|bpe| bpe.encode_with_special_tokens(text).len())
            .unwrap_or_else(|_| fallback(text)),
        "o200k_base" => o200k_base()
            .map(|bpe| bpe.encode_with_special_tokens(text).len())
            .unwrap_or_else(|_| fallback(text)),
        _ => fallback(text),
    }
}

/// Fallback to char estimate if tiktoken fails to load.
fn fallback(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cl100k_counts_hello_world() {
        // "hello world" is 2 tokens under cl100k_base.
        let n = count("hello world", "cl100k_base");
        assert_eq!(n, 2);
    }

    #[test]
    fn o200k_counts_hello_world() {
        // "hello world" is 2 tokens under o200k_base.
        let n = count("hello world", "o200k_base");
        assert_eq!(n, 2);
    }

    #[test]
    fn unknown_encoding_falls_back_to_estimate() {
        // "hello world" = 11 chars → ceil(11/4) = 3.
        let n = count("hello world", "made-up-encoding");
        assert_eq!(n, 3);
    }

    #[test]
    fn empty_string_is_zero() {
        assert_eq!(count("", "cl100k_base"), 0);
    }
}
