use std::path::PathBuf;

use crate::editor::Cursor;

const LINE_CAP: usize = 10;

#[derive(Default)]
pub(crate) struct Buffer {
    pub name: Option<PathBuf>,
    pub content: Vec<String>,
}

impl Buffer {
    pub fn mock() -> Self {
        Self {
            name: None,
            content: vec![String::from("Hello"), String::from("Hi")],
        }
    }

    pub fn line_width(&self, line: usize) -> Option<usize> {
        self.content.get(line).map(|line| line.len())
    }

    pub fn height(&self) -> usize {
        self.content.len()
    }

    pub fn insert_at(&mut self, Cursor { x, y }: Cursor, ch: char) {
        if let Some(line) = self.content.get_mut(y) {
            line.insert(x, ch)
        }
    }

    pub fn new_line(&mut self, at: usize) {
        self.content.insert(at, String::with_capacity(LINE_CAP));
    }

    pub fn break_line(&mut self, Cursor { x, y }: Cursor) {
        if let Some(line) = self.content.get_mut(y) {
            let new_line = line[x..].to_owned();
            line.truncate(x);
            self.content.insert(y + 1, new_line);
        }
    }

    pub fn delete_at(&mut self, Cursor { x, y }: Cursor) {
        if let Some(line) = self.content.get_mut(y) {
            line.remove(x);
        }
    }

    pub fn concat_lines(&mut self, l1: usize, l2: usize) {
        let l1 = self.content.remove(l1);

        if let Some(l2) = self.content.get_mut(l2) {
            l2.push_str(&l1)
        }
    }

    pub fn delete_line(&mut self, line: usize) {
        self.content.remove(line);
    }
}
