use std::{
    cmp::min,
    io::{self, Stdout, Write},
};

use anyhow::Ok;
use crossterm::{
    cursor,
    event::{self, KeyCode, ModifierKeyCode},
    style,
    terminal::{self, disable_raw_mode, enable_raw_mode},
    ExecutableCommand, QueueableCommand,
};

use crate::buffer::Buffer;

#[derive(Clone, Copy)]
enum Mode {
    Normal,
    Insert,
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

enum Action {
    Input(char),
    Line(Direction),
    Move(Direction),
    Change(Mode),
    Quit,
}

pub struct Editor {
    buffers: Vec<Buffer>,
    current_buf_idx: usize,
    mode: Mode,
    cursor: (u16, u16),
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            current_buf_idx: 0,
            buffers: vec![Buffer::mock()],
            mode: Mode::Normal,
            cursor: (0, 0),
        }
    }
}

impl Editor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn execute(&mut self) -> anyhow::Result<()> {
        let mut stdout = io::stdout();

        stdout.execute(terminal::EnterAlternateScreen)?;
        stdout.execute(terminal::Clear(terminal::ClearType::All))?;

        enable_raw_mode()?;

        self.draw_buffer(&mut stdout)?;

        loop {
            self.move_cursor(&mut stdout)?;
            stdout.flush()?;

            let event = event::read()?;

            let Some(action) = (match self.mode {
                Mode::Normal => Normal::handle(event),
                Mode::Insert => Insert::handle(event),
            }) else {
                continue;
            };

            match (action, self.mode) {
                (Action::Move(direction), _) => self.handle_cursor_movment(direction),
                (Action::Quit, _) => break,
                (Action::Change(mode), _) => self.mode = mode,
                (Action::Input(ch), Mode::Insert) => {
                    let cursor = self.cursor;

                    if let Some(buf) = self.current_buf_mut() {
                        buf.insert_at(cursor, ch);
                        self.handle_cursor_movment(Direction::Right);
                        self.draw_buffer(&mut stdout)?;
                    }
                }
                (Action::Input(_), _) => unreachable!(),
                (Action::Line(direction), Mode::Normal) => {
                    let cursor = self.cursor;

                    if let Some(buf) = self.current_buf_mut() {
                        match direction {
                            Direction::Up => {
                                buf.new_line((cursor.1) as usize);
                            }
                            Direction::Down => {
                                buf.new_line((cursor.1 + 1) as usize);
                                self.handle_cursor_movment(Direction::Down);
                            }
                            Direction::Left => unreachable!(),
                            Direction::Right => unreachable!(),
                        }

                        self.draw_buffer(&mut stdout)?;
                        self.mode = Mode::Insert;
                    }
                }
                (Action::Line(_), _) => unreachable!(),
            };
        }

        disable_raw_mode()?;

        stdout.execute(terminal::LeaveAlternateScreen)?;

        Ok(())
    }

    fn draw_buffer(&self, stdout: &mut Stdout) -> anyhow::Result<()> {
        let Some(current_buffer) = self.current_buf() else {
            return Ok(());
        };

        for (idx, line) in current_buffer.content.iter().enumerate() {
            stdout.queue(cursor::MoveTo(0, idx as u16))?;
            stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
            stdout.queue(style::Print(line))?;
        }

        self.move_cursor(stdout)?;

        Ok(())
    }

    fn handle_cursor_movment(&mut self, direction: Direction) {
        let Some(current_buffer) = self.current_buf() else {
            return;
        };

        match direction {
            Direction::Up => self.cursor.1 = self.cursor.1.saturating_sub(1),
            Direction::Down => {
                let line = min(current_buffer.height() as u16, self.cursor.1 + 1);
                let width = current_buffer.line_width(line as usize).unwrap_or(0) as u16;

                self.cursor.1 = line;
                self.cursor.0 = min(width, self.cursor.0);
            }
            Direction::Left => self.cursor.0 = self.cursor.0.saturating_sub(1),
            Direction::Right => {
                let width = current_buffer
                    .line_width(self.cursor.1 as usize)
                    .unwrap_or(0) as u16;

                self.cursor.0 = min(width, self.cursor.0 + 1)
            }
        }
    }

    fn move_cursor(&self, stdout: &mut Stdout) -> anyhow::Result<()> {
        stdout.queue(cursor::MoveTo(self.cursor.0, self.cursor.1))?;

        Ok(())
    }

    fn current_buf(&self) -> Option<&Buffer> {
        self.buffers.get(self.current_buf_idx)
    }

    fn current_buf_mut(&mut self) -> Option<&mut Buffer> {
        self.buffers.get_mut(self.current_buf_idx)
    }
}

trait HandleEvent {
    fn handle(event: event::Event) -> Option<Action>;
}

struct Normal;

impl HandleEvent for Normal {
    fn handle(event: event::Event) -> Option<Action> {
        match event {
            event::Event::Key(event) => match event.code {
                KeyCode::Char('j') => Some(Action::Move(Direction::Down)),
                KeyCode::Char('k') => Some(Action::Move(Direction::Up)),
                KeyCode::Char('h') => Some(Action::Move(Direction::Left)),
                KeyCode::Char('l') => Some(Action::Move(Direction::Right)),
                KeyCode::Char('i') => Some(Action::Change(Mode::Insert)),
                KeyCode::Char('O') => Some(Action::Line(Direction::Up)),
                KeyCode::Char('o') => Some(Action::Line(Direction::Down)),
                KeyCode::Char('q') => Some(Action::Quit),
                _ => None,
            },
            _ => None,
        }
    }
}

struct Insert;

impl HandleEvent for Insert {
    fn handle(event: event::Event) -> Option<Action> {
        match event {
            event::Event::Key(event) => match event.code {
                KeyCode::Esc => Some(Action::Change(Mode::Normal)),
                KeyCode::Char(ch) => Some(Action::Input(ch)),
                _ => None,
            },
            _ => None,
        }
    }
}
