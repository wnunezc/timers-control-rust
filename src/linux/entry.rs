//! Edición UTF-8 sin dependencia del compositor ni de la representación gráfica.
#![forbid(unsafe_code)]

#[derive(Default)]
pub struct Entry {
    pub text: String,
    pub cursor: usize,
    pub selected: bool,
}

impl Entry {
    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.selected = false;
    }
    pub fn restore(&mut self, text: String) {
        self.text = text;
        self.cursor = self.text.len();
        self.selected = false;
    }
    pub fn left(&mut self) {
        self.cursor = if self.selected {
            0
        } else {
            self.text[..self.cursor]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i)
        };
        self.selected = false;
    }
    pub fn right(&mut self) {
        self.cursor = if self.selected {
            self.text.len()
        } else {
            self.cursor
                + self.text[self.cursor..]
                    .chars()
                    .next()
                    .map_or(0, char::len_utf8)
        };
        self.selected = false;
    }
    pub fn backspace(&mut self) {
        if self.selected {
            self.clear();
            return;
        }
        let end = self.cursor;
        self.left();
        self.text.drain(self.cursor..end);
    }
    pub fn delete(&mut self) {
        if self.selected {
            self.clear();
        } else if self.cursor < self.text.len() {
            self.text.remove(self.cursor);
        }
    }
    pub fn insert(&mut self, text: &str) {
        if text.chars().any(char::is_control) {
            return;
        }
        let units = text.encode_utf16().count();
        let current = if self.selected {
            0
        } else {
            self.text.encode_utf16().count()
        };
        if current + units > 1024 {
            return;
        }
        if self.selected {
            self.clear();
        }
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edits_unicode_without_breaking_boundaries() {
        let mut entry = Entry::default();
        entry.insert("café💡");
        entry.left();
        entry.backspace();
        assert_eq!(entry.text, "caf💡");
        entry.delete();
        assert_eq!(entry.text, "caf");
        entry.selected = true;
        entry.insert("nuevo");
        assert_eq!(entry.text, "nuevo");
    }
    #[test]
    fn enforces_input_limit_and_rejects_controls() {
        let mut entry = Entry::default();
        entry.insert(&"a".repeat(1024));
        entry.insert("b");
        assert_eq!(entry.text.len(), 1024);
        entry.selected = true;
        entry.insert("\0");
        assert_eq!(entry.text.len(), 1024);
    }
}
