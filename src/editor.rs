use std::{
    cmp::min,
    io::{self, Stdout, Write},
    ops::{Add, Sub},
};

use anyhow::Ok;
use crossterm::{
    cursor,
    event::{self, KeyCode},
    style,
    terminal::{self, disable_raw_mode, enable_raw_mode},
    ExecutableCommand, QueueableCommand,
};

use crate::buffer::Buffer;

#[derive(Default, Copy, Clone)]
pub struct Cursor {
    pub x: usize,
    pub y: usize,
}

impl Cursor {
    fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

impl Add for Cursor {
    type Output = Cursor;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Cursor {
    type Output = Cursor;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

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
    Change(Mode, Option<Direction>),
    Delete,
    DeleteLine,
    Quit,
}

pub struct Editor {
    buffers: Vec<Buffer>,
    current_buf_idx: usize,
    mode: Mode,
    cursor: Cursor,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            current_buf_idx: 0,
            buffers: vec![Buffer::mock()],
            mode: Mode::Normal,
            cursor: Cursor::default(),
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
                (Action::Change(mode, Some(direction)), _) => {
                    self.mode = mode;
                    self.handle_cursor_movment(direction)
                }
                (Action::Change(mode, None), _) => self.mode = mode,
                (Action::Delete, Mode::Insert) if self.cursor.x == 0 && self.cursor.y > 0 => {
                    stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;

                    self.handle_cursor_movment(Direction::Up);
                    self.move_cursor_end_of_the_line();
                    let cursor = self.cursor;

                    if let Some(buf) = self.current_buf_mut() {
                        buf.concat_lines(cursor.y + 1, cursor.y);
                        self.clear_last_line(&mut stdout)?;
                        self.draw_buffer(&mut stdout)?;
                    }
                }
                (Action::Delete, Mode::Insert) if self.cursor.x > 0 => {
                    let cursor = self.cursor;

                    if let Some(buf) = self.current_buf_mut() {
                        buf.delete_at(cursor - Cursor::new(1, 0));
                        self.handle_cursor_movment(Direction::Left);
                        self.draw_buffer(&mut stdout)?;
                    }
                }
                (Action::Delete, _) => {}
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
                                buf.new_line(cursor.y);
                            }
                            Direction::Down => {
                                buf.new_line(cursor.y + 1);
                                self.handle_cursor_movment(Direction::Down);
                            }
                            Direction::Left => unreachable!(),
                            Direction::Right => unreachable!(),
                        }

                        self.draw_buffer(&mut stdout)?;
                        self.mode = Mode::Insert;
                    }
                }
                (Action::Line(direction), Mode::Insert) => {
                    let cursor = self.cursor;

                    if let Some(buf) = self.current_buf_mut() {
                        match direction {
                            Direction::Up => {
                                buf.break_line(cursor);
                            }
                            Direction::Down => {
                                buf.break_line(cursor);
                                self.handle_cursor_movment(Direction::Down);
                                self.move_cursor_start_of_the_line();
                            }
                            Direction::Left => unreachable!(),
                            Direction::Right => unreachable!(),
                        }

                        self.draw_buffer(&mut stdout)?;
                        self.mode = Mode::Insert;
                    }
                }
                (Action::DeleteLine, Mode::Normal) => {
                    let cursor = self.cursor;

                    if let Some(buf) = self.current_buf_mut() {
                        buf.delete_line(cursor.y);
                        self.clear_last_line(&mut stdout)?;
                        self.handle_cursor_movment(Direction::Up);
                        self.draw_buffer(&mut stdout)?;
                    }
                }
                (Action::DeleteLine, _) => todo!(),
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

    fn move_cursor_start_of_the_line(&mut self) {
        self.cursor.x = 0;
    }

    fn move_cursor_end_of_the_line(&mut self) {
        if let Some(current_buffer) = self.current_buf() {
            self.cursor.x = current_buffer.line_width(self.cursor.x).unwrap_or(0);
        };
    }

    fn handle_cursor_movment(&mut self, direction: Direction) {
        let Some(current_buffer) = self.current_buf() else {
            return;
        };

        match direction {
            Direction::Up => {
                let line = self.cursor.y.saturating_sub(1);
                let width = current_buffer
                    .line_width(line)
                    .unwrap_or(0)
                    .saturating_sub(1);

                self.cursor.y = line;
                self.cursor.x = min(width, self.cursor.x);
            }
            Direction::Down => {
                let line = min(current_buffer.height().saturating_sub(1), self.cursor.y + 1);
                let width = current_buffer
                    .line_width(line)
                    .unwrap_or(0)
                    .saturating_sub(1);

                self.cursor.y = line;
                self.cursor.x = min(width, self.cursor.x);
            }
            Direction::Left => self.cursor.x = self.cursor.x.saturating_sub(1),
            Direction::Right => {
                let mut width = current_buffer.line_width(self.cursor.y).unwrap_or(0);

                if matches!(self.mode, Mode::Normal) {
                    width -= 1;
                }

                self.cursor.x = min(width, self.cursor.x + 1)
            }
        }
    }

    fn move_cursor(&self, stdout: &mut Stdout) -> anyhow::Result<()> {
        stdout.queue(cursor::MoveTo(self.cursor.x as u16, self.cursor.y as u16))?;

        Ok(())
    }

    fn current_buf(&self) -> Option<&Buffer> {
        self.buffers.get(self.current_buf_idx)
    }

    fn current_buf_mut(&mut self) -> Option<&mut Buffer> {
        self.buffers.get_mut(self.current_buf_idx)
    }

    fn clear_last_line(&self, stdout: &mut Stdout) -> anyhow::Result<()> {
        if let Some(current_buffer) = self.current_buf() {
            stdout.queue(cursor::MoveTo(0, current_buffer.height() as u16))?;
            stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;

            self.move_cursor(stdout)?;
        };

        Ok(())
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
                KeyCode::Char('i') => Some(Action::Change(Mode::Insert, None)),
                KeyCode::Char('a') => Some(Action::Change(Mode::Insert, Some(Direction::Right))),
                KeyCode::Char('O') => Some(Action::Line(Direction::Up)),
                KeyCode::Char('o') => Some(Action::Line(Direction::Down)),
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Char('D') => Some(Action::DeleteLine),
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
                KeyCode::Esc => Some(Action::Change(Mode::Normal, Some(Direction::Left))),
                KeyCode::Char('[') if event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    Some(Action::Change(Mode::Normal, Some(Direction::Left)))
                }
                KeyCode::Enter => Some(Action::Line(Direction::Down)),
                KeyCode::Backspace => Some(Action::Delete),
                KeyCode::Char(ch) => Some(Action::Input(ch)),
                _ => None,
            },
            _ => None,
        }
    }
}
