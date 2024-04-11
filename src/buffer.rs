use std::{cmp::min, ops::Deref, path::PathBuf};

use crate::{
    editor::{Direction, Mode},
    pos::Pos,
};

const LINE_CAP: usize = 10;

#[derive(Default)]
pub(crate) struct Buffer {
    pub name: Option<PathBuf>,
    pub content: Vec<String>,
    pub cursor: Pos,
}

impl Buffer {
    pub fn mock() -> Self {
        Self {
            name: None,
            content: vec![String::from("Hello"), String::from("Hi")],
            cursor: Pos::new(0, 0),
        }
    }

    pub fn line_width(&self, line: usize) -> Option<usize> {
        self.content.get(line).map(|line| line.len())
    }

    pub fn current_line_width(&self) -> Option<usize> {
        self.line_width(self.cursor.y)
    }

    pub fn height(&self) -> usize {
        self.content.len()
    }

    pub fn insert_at(&mut self, ch: char) {
        let Pos { x, y } = self.cursor;

        if let Some(line) = self.content.get_mut(y) {
            line.insert(x, ch)
        }
    }

    pub fn new_line(&mut self, at: usize) {
        self.content.insert(at, String::with_capacity(LINE_CAP));
    }

    pub fn break_line(&mut self) {
        let Pos { x, y } = self.cursor;

        if let Some(line) = self.content.get_mut(y) {
            let new_line = line[x..].to_owned();
            line.truncate(x);
            self.content.insert(y + 1, new_line);
        }
    }

    pub fn delete_at(&mut self, direction: Option<Direction>) {
        let Pos { x, y } = match direction {
            Some(Direction::Up) => self.cursor - Pos::new(0, 1),
            Some(Direction::Down) => self.cursor + Pos::new(0, 1),
            _ => self.cursor,
        };

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

    pub fn move_cursor_start_of_the_line(&mut self) {
        self.cursor.x = 0;
    }

    pub fn move_cursor_end_of_the_line(&mut self) {
        self.cursor.x = self.current_line_width().unwrap_or(0);
    }

    pub fn handle_cursor_movment(&mut self, mode: Mode, direction: Direction) {
        match direction {
            Direction::Up => {
                let line = self.cursor.y.saturating_sub(1);
                let width = self.line_width(line).unwrap_or(0).saturating_sub(1);

                self.cursor.y = line;
                self.cursor.x = min(width, self.cursor.x);
            }
            Direction::Down => {
                let line = min(self.height().saturating_sub(1), self.cursor.y + 1);
                let width = self.line_width(line).unwrap_or(0).saturating_sub(1);

                self.cursor.y = line;
                self.cursor.x = min(width, self.cursor.x);
            }
            Direction::Left => self.cursor.x = self.cursor.x.saturating_sub(1),
            Direction::Right => {
                let mut width = self.line_width(self.cursor.y).unwrap_or(0);

                if matches!(mode, Mode::Normal) {
                    width -= 1;
                }

                self.cursor.x = min(width, self.cursor.x + 1)
            }
        }
    }
}

#[derive(Default)]
struct CommandBuffer(Buffer);

impl Deref for CommandBuffer {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CommandBuffer {}
