use std::{io::{Stdin, Read, stdout, Write}, process, path::PathBuf, fs::{File, OpenOptions}, os::fd::AsRawFd, fmt::Display};

use anyhow::Error;
use raw_tty::TtyWithGuard;
use thiserror::Error;
use libc::{winsize, ioctl, TIOCGWINSZ};

#[derive(Error, Debug)]
pub enum EditorError {
}

#[derive(Eq, PartialEq)]
pub enum Mode {
    Normal,
    NormalCommandBuffer,
    NormalComposing(ComposableCommand),
    Insert,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal | Self::NormalComposing(_) | Self::NormalCommandBuffer => {
                write!(f, "NORMAL")
            },
            Self::Insert => {
                write!(f, "INSERT")
            }
        }
    }
}


pub enum Motion {
    Word(usize),
    BackWord(usize),
    Forward(char, usize),
    EOL,
    Chars(usize),
}

#[derive(Eq, PartialEq)]
pub enum ComposableCommand {
    Delete,
    Yank,
    Change,
}

struct Cursor {
    pub x: usize,
    pub y: usize,
}

pub struct Editor {
    stdin: TtyWithGuard<Stdin>,
    mode: Mode,
    working: PathBuf,
    command_buffer: String,
    internal: Vec<u8>,
    cursor: Cursor,
    num_buf: String,
}

impl Editor {
    pub fn new(stdin: TtyWithGuard<Stdin>, working: PathBuf) -> Self {
        let mut file = OpenOptions::new().read(true).write(true).create(true).open(working.clone()).unwrap();
        let mut internal = Vec::with_capacity(file.metadata().unwrap().len() as usize);
        file.read_to_end(&mut internal).unwrap();
        Self {
            stdin,
            mode: Mode::Normal,
            command_buffer: String::with_capacity(100),
            working,
            internal,
            cursor: Cursor { x: 0, y: 0 },
            num_buf: String::with_capacity(5),
        }
    }

    pub fn run(&mut self) -> Result<(), Error> {
        loop {
            self.render();
            let mut single = vec![0u8; 1];
            self.stdin.read_exact(&mut single)?;
            let byte = single.get(0).unwrap();

            match &self.mode {
                Mode::Normal => {
                    let mut amt = 1;
                    if !self.num_buf.is_empty() && !(b'0'..=b'9').contains(byte) {
                        amt = self.num_buf.parse().unwrap();
                        self.num_buf.clear();
                    }
                    match byte {
                        b':' => {
                            self.mode = Mode::NormalCommandBuffer;
                        },
                        b'i' => {
                            self.mode = Mode::Insert;
                        },
                        b'l' => {
                            self.cursor_right(amt);
                        },
                        b'h' => {
                            self.cursor_left(amt);
                        },
                        b'j' => {
                            self.cursor_down(amt); 
                        },
                        b'k' => {
                            self.cursor_up(amt);
                        },
                        b'x' => {
                            self.remove(amt);
                        },
                        b'0'..=b'9' => {
                            self.num_buf.push(*byte as char);
                        }
                        _ => {},
                    }
                },
                Mode::NormalCommandBuffer => {
                    match byte {
                        b'\x0d' => {
                            self.execute_command_buffer();
                        },
                        b'\x1b' => {
                            self.cancel_command_buffer();
                        },
                        0x08 | 0x7F => {
                            self.command_buffer.pop();
                        }
                        a => {
                            self.command_buffer.push(*a as char);
                        },
                    }
                },
                Mode::NormalComposing(cmd) => {
                }
                Mode::Insert => {
                    match byte {
                        b'\x1b' => {
                            self.mode = Mode::Normal;
                        },
                        a => {
                            self.insert_at_cursor(*a, 0);
                        },
                    }
                }
            }
        }
    }

    fn render(&mut self) {
        print!("\r\x1b[2J");
        print!("\r\x1b[H");
        self.print_internal();
        self.sync_cursor();
        self.print_line_nums();
        self.print_statusline();
        self.sync_cursor();

        stdout().flush().unwrap();
    }

    fn execute_command_buffer(&mut self) {
        if self.command_buffer == "q" {
            process::exit(0);
        } else if self.command_buffer == "w" {
            let mut file = OpenOptions::new().read(true).write(true).create(true).truncate(true).open(self.working.clone()).unwrap();
            file.write_all(&self.internal).unwrap();
        }
        self.clear_command_buffer();
    }

    fn cancel_command_buffer(&mut self) {
        self.clear_command_buffer();
    }

    fn clear_command_buffer(&mut self) {
        self.mode = Mode::Normal;
        self.command_buffer.clear();
    }

    fn cursor_left(&mut self, amt: usize) {
        self.cursor.x = self.cursor.x.saturating_sub(amt);
    }

    fn cursor_right(&mut self, amt: usize) {
        self.cursor.x = self.cursor.x.saturating_add(amt);
    }

    fn cursor_up(&mut self, amt: usize) {
        self.cursor.y = self.cursor.y.saturating_sub(amt);
    }

    fn cursor_down(&mut self, amt: usize) {
        self.cursor.y = self.cursor.y.saturating_add(amt);
    }

    fn length_at_cursor(&self) -> usize {
        let mut lines = self.internal.rsplit(|val| *val == b'\n').rev(); //no clue why these are
                                                                         //backwards..
        return lines.nth(self.cursor.y - 1).unwrap().len();
    }

    fn idx_at_cursor(&self) -> usize {
        let mut idx: usize = 0;
        let lines = self.internal.rsplit(|val| *val == b'\n').rev();
        for (i, line) in lines.enumerate() {
            if i == (self.cursor.y).saturating_sub(1) {
                idx += (self.cursor.x).saturating_sub(1);
                break;
            }
            idx += line.len() + 1;
        }
        return idx;
    }

    fn n_lines(&self) -> usize {
        return self.internal.rsplit(|val| *val == b'\n').count();
    }

    fn sync_cursor(&mut self) {
        if self.cursor.y <= 0 {
            self.cursor.y = 1;
        }
        if self.cursor.x <= 0 {
            self.cursor.x = 1;
        }
        if self.cursor.y as usize > self.n_lines() - 1{
            self.cursor.y = self.n_lines() - 1;
        }
        if self.cursor.x as usize > self.length_at_cursor() {
            self.cursor.x = self.length_at_cursor();
        }
        
        print!("\x1b[{};{}H", self.cursor.y, self.cursor.x + 4);
        stdout().flush().unwrap();
    }

    fn print_internal(&mut self) {
        print!("    ");
        for b in &self.internal {
            match b {
                b'\n' => {
                    stdout().write_all(&[b'\n', b'\r']);
                    print!("    ");
                }
                a => {
                    stdout().write_all(&[*a]);
                }
            }
        }
    }

    fn remove(&mut self, amt: usize) {
        for _ in 0..(amt.min(self.length_at_cursor() - self.cursor.x + 1)) {
            self.internal.remove(self.idx_at_cursor());
        }
    }

    fn insert_at_cursor(&mut self, char: u8, offset: usize) {
        if char == b'\r' || char == b'\n' {
            self.internal.insert(self.idx_at_cursor() + offset, b'\n');
            self.cursor_down(1);
            self.cursor.x = 0;
            return;
        } else if char == 0x08 || char == 0x7F {
            self.remove(1);
            self.cursor_left(1);
            return;
        } else if char == 0x09 || char == 0x0B {
            for _ in 0..4 {
                self.cursor_right(1);
                self.internal.insert(self.idx_at_cursor() + offset, b' ');
            }
            return;
        }
        self.cursor_right(1);
        self.internal.insert(self.idx_at_cursor() + offset, char);
    }

    fn print_statusline(&self) {
        let winsize = unsafe { self.get_winsize() };
        print!("\x1b[{};0H", winsize.ws_row);
        print!("  \x1b[1m{}\x1b[0m", self.mode);
        let cmd_buf = if self.mode != Mode::NormalCommandBuffer {
            "".to_string()
        } else {
            format!(":{}", self.command_buffer)
        };
        print!("\x1b[{};{}H", winsize.ws_row, winsize.ws_col / 2 - (cmd_buf.len() as u16 / 2));
        print!("{}", cmd_buf);
        let pos = format!("{},{}", self.cursor.x, self.cursor.y);
        print!("\x1b[{};{}H", winsize.ws_row, winsize.ws_col - (pos.len() as u16 + 2));
        print!("{}", pos);
    }

    fn print_line_nums(&self) {
        for i in 0..self.n_lines() {
            let line_number = if (i + 1 == self.cursor.y) {
                let out = (i + 1).to_string();
                print!("\x1b[{};0H", i + 1);
                out
            } else if (i + 1) > self.cursor.y {
                let out = ((i + 1) - self.cursor.y).to_string();
                print!("\x1b[{};{}H", i + 1, 4 - out.len());
                out
            } else {
                let out = (self.cursor.y - (i + 1)).to_string();
                print!("\x1b[{};{}H", i + 1, 4 - out.len());
                out
            };
            print!("\x1b[38;5;245m{}\x1b[0m", line_number);
        }
    }

    unsafe fn get_winsize(&self) -> winsize {
        let winsize = winsize {
            ws_col: 0,
            ws_row: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        ioctl(stdout().as_raw_fd(), TIOCGWINSZ, &winsize);
        return winsize;
    }
}
