use std::path::PathBuf;

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

    pub fn insert_at(&mut self, (x, y): (u16, u16), ch: char) {
        if let Some(line) = self.content.get_mut(y as usize) {
            line.insert(x as usize, ch)
        }
    }

    pub fn new_line(&mut self, at: usize) {
        self.content.insert(at, String::with_capacity(LINE_CAP));
    }
}
