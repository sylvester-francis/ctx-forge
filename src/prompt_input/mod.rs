//! Multi-line text input widget for the task prompt.
//!
//! Source of truth: a `String` plus byte-offset cursor. Cursor movement is
//! char-level (no combining-mark handling). Mutators keep the cursor on a
//! valid char boundary and within `text.len()`.

#![allow(dead_code)]

pub mod at_picker;

#[derive(Debug, Clone, Default)]
pub struct PromptInput {
    text: String,
    cursor: usize,
}

impl PromptInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_text(text: String) -> Self {
        let cursor = text.len();
        Self { text, cursor }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn line_count(&self) -> usize {
        self.text.split('\n').count()
    }

    pub fn set_text(&mut self, text: String) {
        self.cursor = text.len();
        self.text = text;
    }

    pub fn take_text(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.text)
    }

    pub fn set_cursor(&mut self, pos: usize) {
        let mut c = pos.min(self.text.len());
        while c > 0 && !self.text.is_char_boundary(c) {
            c -= 1;
        }
        self.cursor = c;
    }

    pub fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn insert_str(&mut self, s: &str) {
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let prev = self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.text.replace_range(prev..self.cursor, "");
        self.cursor = prev;
    }

    /// Delete back to the start of the previous word. Trailing whitespace
    /// is collapsed in, so Ctrl-W on "foo   " removes both the spaces and "foo".
    pub fn delete_word_back(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let before = &self.text[..self.cursor];
        let trimmed = before.trim_end_matches(char::is_whitespace);
        let word_start = trimmed
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_whitespace())
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        self.text.replace_range(word_start..self.cursor, "");
        self.cursor = word_start;
    }

    pub fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor = self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }

    pub fn move_right(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let next = self.text[self.cursor..].chars().next().unwrap();
        self.cursor += next.len_utf8();
    }

    pub fn move_home(&mut self) {
        let before = &self.text[..self.cursor];
        self.cursor = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    }

    pub fn move_end(&mut self) {
        let after = &self.text[self.cursor..];
        self.cursor += after.find('\n').unwrap_or(after.len());
    }

    /// Byte-based column within the current line. Callers needing grid
    /// columns should apply `UnicodeWidthStr` for non-ASCII.
    pub fn cursor_line_column(&self) -> (u16, u16) {
        let before = &self.text[..self.cursor];
        let line_no = before.bytes().filter(|&b| b == b'\n').count() as u16;
        let last_nl = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = (self.cursor - last_nl) as u16;
        (col, line_no)
    }
}
