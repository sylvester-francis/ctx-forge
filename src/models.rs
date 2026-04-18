//! Model registry: name → (tokenizer, context window).
//!
//! ctxforge uses this to decide which tokenizer to apply and to compute the
//! "percentage of budget" figure shown in `ctxforge status` and the TUI gauge.

#![allow(dead_code)]

/// Which tokenizer to use for a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tokenizer {
    /// Exact tiktoken via `tiktoken-rs`.
    Tiktoken(&'static str), // e.g. "cl100k_base", "o200k_base"
    /// chars / 4 heuristic.
    Estimate,
}

#[derive(Debug, Clone, Copy)]
pub struct ModelInfo {
    pub name: &'static str,
    pub window: usize,
    pub tokenizer: Tokenizer,
}

pub const DEFAULT_MODEL: &str = "claude-sonnet-4";

/// All models ctxforge knows about. Unknown names fall back to `estimate` with
/// a generic 200k window.
const MODELS: &[ModelInfo] = &[
    // ── Anthropic (Claude 4.6 / 4.5 / 4) ───────────────────────────
    ModelInfo {
        name: "claude-opus-4-6",
        window: 1_000_000,
        tokenizer: Tokenizer::Estimate,
    },
    ModelInfo {
        name: "claude-sonnet-4-6",
        window: 200_000,
        tokenizer: Tokenizer::Estimate,
    },
    ModelInfo {
        name: "claude-haiku-4-5",
        window: 200_000,
        tokenizer: Tokenizer::Estimate,
    },
    ModelInfo {
        name: "claude-sonnet-4",
        window: 200_000,
        tokenizer: Tokenizer::Estimate,
    },
    ModelInfo {
        name: "claude-opus-4",
        window: 200_000,
        tokenizer: Tokenizer::Estimate,
    },
    ModelInfo {
        name: "claude-3-5-sonnet",
        window: 200_000,
        tokenizer: Tokenizer::Estimate,
    },
    // ── OpenAI (exact tiktoken) ─────────────────────────────────────
    ModelInfo {
        name: "gpt-4.1",
        window: 1_047_576,
        tokenizer: Tokenizer::Tiktoken("o200k_base"),
    },
    ModelInfo {
        name: "gpt-4.1-mini",
        window: 1_047_576,
        tokenizer: Tokenizer::Tiktoken("o200k_base"),
    },
    ModelInfo {
        name: "gpt-4.1-nano",
        window: 1_047_576,
        tokenizer: Tokenizer::Tiktoken("o200k_base"),
    },
    ModelInfo {
        name: "o4-mini",
        window: 200_000,
        tokenizer: Tokenizer::Tiktoken("o200k_base"),
    },
    ModelInfo {
        name: "o3",
        window: 200_000,
        tokenizer: Tokenizer::Tiktoken("o200k_base"),
    },
    ModelInfo {
        name: "o3-mini",
        window: 200_000,
        tokenizer: Tokenizer::Tiktoken("o200k_base"),
    },
    ModelInfo {
        name: "o1",
        window: 200_000,
        tokenizer: Tokenizer::Tiktoken("o200k_base"),
    },
    ModelInfo {
        name: "gpt-4o",
        window: 128_000,
        tokenizer: Tokenizer::Tiktoken("o200k_base"),
    },
    ModelInfo {
        name: "gpt-4o-mini",
        window: 128_000,
        tokenizer: Tokenizer::Tiktoken("o200k_base"),
    },
    ModelInfo {
        name: "gpt-4-turbo",
        window: 128_000,
        tokenizer: Tokenizer::Tiktoken("cl100k_base"),
    },
    // ── Google ──────────────────────────────────────────────────────
    ModelInfo {
        name: "gemini-2.5-pro",
        window: 1_048_576,
        tokenizer: Tokenizer::Estimate,
    },
    ModelInfo {
        name: "gemini-2.5-flash",
        window: 1_048_576,
        tokenizer: Tokenizer::Estimate,
    },
    ModelInfo {
        name: "gemini-2-flash",
        window: 1_000_000,
        tokenizer: Tokenizer::Estimate,
    },
    ModelInfo {
        name: "gemini-1.5-pro",
        window: 2_000_000,
        tokenizer: Tokenizer::Estimate,
    },
    ModelInfo {
        name: "gemini-1.5-flash",
        window: 1_000_000,
        tokenizer: Tokenizer::Estimate,
    },
];

/// Return the full list of known models. Used by the TUI model switcher.
pub fn all_models() -> &'static [ModelInfo] {
    MODELS
}

/// Model ids as a `Vec<String>` — used by TUI picker overlays.
pub fn list_ids() -> Vec<String> {
    MODELS.iter().map(|m| m.name.to_string()).collect()
}

/// Lookup by name. Unknown names return a fallback with 200k window and
/// estimate tokenizer so ctxforge degrades gracefully.
pub fn lookup(name: &str) -> ModelInfo {
    MODELS
        .iter()
        .find(|m| m.name.eq_ignore_ascii_case(name))
        .copied()
        .unwrap_or(ModelInfo {
            name: "unknown",
            window: 200_000,
            tokenizer: Tokenizer::Estimate,
        })
}

pub fn default_model() -> ModelInfo {
    lookup(DEFAULT_MODEL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_known_model() {
        let m = lookup("gpt-4o");
        assert_eq!(m.window, 128_000);
        assert!(matches!(m.tokenizer, Tokenizer::Tiktoken(_)));
    }

    #[test]
    fn lookup_unknown_falls_back() {
        let m = lookup("nonexistent-model-xyz");
        assert_eq!(m.name, "unknown");
        assert_eq!(m.window, 200_000);
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(lookup("GPT-4o").window, 128_000);
    }

    #[test]
    fn default_model_is_claude_sonnet() {
        assert_eq!(default_model().name, "claude-sonnet-4");
    }
}
