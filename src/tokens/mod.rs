//! Token counting dispatcher. Uses the model's declared tokenizer.

#![allow(dead_code)]

pub mod estimate;
pub mod tiktoken;

use crate::models::{ModelInfo, Tokenizer};

/// A counted token total. `exact = false` means it's a char-based estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenCount {
    pub tokens: usize,
    pub exact: bool,
}

impl TokenCount {
    /// Format as `"1,204"` (exact) or `"~1,204"` (estimate).
    pub fn format(&self) -> String {
        let sep = thousands(self.tokens);
        if self.exact {
            sep
        } else {
            format!("~{sep}")
        }
    }
}

fn thousands(n: usize) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

/// Count tokens in `text` for the given model.
pub fn count(text: &str, model: &ModelInfo) -> TokenCount {
    match model.tokenizer {
        Tokenizer::Estimate => TokenCount {
            tokens: estimate::count(text),
            exact: false,
        },
        Tokenizer::Tiktoken(encoding) => TokenCount {
            tokens: tiktoken::count(text, encoding),
            exact: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_formats() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(12_345), "12,345");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }

    #[test]
    fn format_exact_has_no_tilde() {
        assert_eq!(
            TokenCount {
                tokens: 1234,
                exact: true
            }
            .format(),
            "1,234"
        );
    }

    #[test]
    fn format_estimate_has_tilde() {
        assert_eq!(
            TokenCount {
                tokens: 1234,
                exact: false
            }
            .format(),
            "~1,234"
        );
    }
}
