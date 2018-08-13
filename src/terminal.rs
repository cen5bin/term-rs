use pancurses::{Window, initscr, noecho, Input, resize_term};
use super::command::CommandHistory;

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
struct Position(i32, i32);

pub struct Terminal<F> {
    prompt: String,
    window: Window,
    history: CommandHistory,
    buf: Vec<u8>,
    pos: i32,
    process: F,
}

impl<F> Terminal<F>
    where F: Fn(String) -> String {
    pub fn run(process: F) {
        let window = initscr();
        window.keypad(true);
        window.scrollok(true);
        window.setscrreg(0, window.get_max_y());
        noecho();
        let mut t = Terminal {
            prompt: "debug> ".to_owned(),
            window,
            history: CommandHistory::default(),
            buf: Vec::new(),
            pos: 0,
            process,
        };
        loop {
            let command = t.input();
            let result = (t.process)(command);
            t.window.printw(format!("{}\n", result));
        }
    }

    fn print_prompt(&self) {
        self.window.printw(self.prompt.as_str());
    }

    fn input(&mut self) -> String {
        self.print_prompt();
        self.pos = 0;
        loop {
            if let Some(ch) = self.window.getch() {
                match ch {
                    Input::Character(c) => {
                        match c {
                            '\n' => { return self.line_feed(); }
                            '\t' => {}
                            '\u{7f}' => { self.backspace(); }
                            '\u{15}' => {
                                // ctrl+U
                                self.clear_to_start();
                            }
                            '\u{c}' => {
                                // ctrl+L
                                self.clear_line();
                            }
                            '\u{1}' => {
                                // ctl+A
                                self.move_to_start();
                            }
                            '\u{5}' => {
                                // ctrl+E
                                self.move_to_end();
                            }
                            x if (x as u8) >= 0x20 && (x as u8) <= 0x7E => { self.insert(x.to_string()); }
                            _ => {}
                        }
                    }
                    Input::KeyBackspace => {self.backspace();}
                    Input::KeyResize => { self.on_resized(); }
                    Input::KeyUp => { self.prev_command(); }
                    Input::KeyDown => { self.next_command(); }
                    Input::KeyLeft => { self.move_left(); }
                    Input::KeyRight => { self.move_right(); }
                    x => { println!("{:?}", x); }
                }
            }
        }
    }

    fn on_resized(&mut self) {
        resize_term(0, 0);
        self.window.setscrreg(0, self.window.get_max_y());
    }

    fn line_feed(&mut self) -> String {
        let ret = String::from_utf8(self.buf.clone()).unwrap();
        self.clear_line();
        self.window.printw(format!("{}\n", ret));
        if ret.trim().len() > 0 {
            self.history.add_command(ret.clone());
        }
        self.pos = 0;
        return ret;
    }

    fn prev_command(&mut self) {
        if self.history.at_top() {
            let command = String::from_utf8(self.buf.clone()).unwrap();
            self.history.add_command(command);
            self.history.prev_command();
        }
        self.clear_line();
        if let Some(command) = self.history.prev_command() {
            self.buf.extend(command.as_bytes());
            self.window.printw(command);
        }
        self.pos = self.buf.len() as i32;
    }

    fn next_command(&mut self) {
        self.clear_line();
        if let Some(command) = self.history.next_command() {
            self.buf.extend(command.as_bytes());
            self.window.printw(command);
        }
        self.pos = self.buf.len() as i32;
    }

    fn insert(&mut self, text: String) {
        if self.pos == self.buf.len() as i32 {
            self.buf.extend(text.as_bytes());
            self.pos += text.as_bytes().len() as i32;
            self.window.printw(text);
        } else {
            let tmp = {
                let pre = &self.buf[0..self.pos as usize];
                let end = &self.buf[self.pos as usize..];
                let mut tmp = Vec::new();
                tmp.extend(pre);
                tmp.extend(text.as_bytes());
                tmp.extend(end);
                tmp
            };
            let len = text.as_bytes().len() as i32;
            let pos = self.pos + len;
            for _ in 0..len {
                self.move_right();
            }
            let position = self.current_position();
            self.clear_line();
            self.buf = tmp;
            self.window.printw(String::from_utf8(self.buf.clone()).unwrap());
            self.pos = pos;
            self.window.mv(position.1, position.0);
        }

    }

    fn clear_to_start(&mut self) {
        let tmp = self.buf[self.pos as usize..].to_owned();
        let origin = self.line_start_position();
        self.clear_line();
        self.buf = tmp;
        self.window.printw(String::from_utf8(self.buf.clone()).unwrap());
        self.window.mv(origin.1, origin.0);
    }

    fn backspace(&mut self) {
        if self.pos == 0 {

        } else if self.pos == self.buf.len() as i32 {
            self.move_left();
            self.window.delch();
            self.buf.pop();
        } else {
            self.move_left();
            self.buf.remove(self.pos as usize);
            let p = self.current_position();
            let pos = self.pos;
            let tmp = self.buf.clone();
            self.clear_line();
            self.buf = tmp;
            self.window.printw(String::from_utf8(self.buf.clone()).unwrap());
            self.window.mv(p.1, p.0);
            self.pos = pos;
        }
    }

    fn clear_line(&mut self) {
        let end_y = self.line_end_position().1;
        let start_y = self.line_start_position().1;
        let mut y = end_y;
        while y >= start_y {
            self.window.mv(y, 0);
            self.window.deleteln();
            y -= 1;
        }
        self.buf.clear();
        self.print_prompt();
        self.pos = 0;
        debug_assert_eq!(self.line_start_position(), self.current_position());
    }

    fn current_position(&self) -> Position {
        Position(self.window.get_cur_x(), self.window.get_cur_y())
    }

    fn move_left(&mut self) {
        if self.pos > 0 {
            let Position(x, y) = self.current_position();
            if x == 0 {
                self.window.mv(y - 1, self.window.get_max_x() - 1);
            } else {
                self.window.mv(y, x - 1);
            }
            self.pos -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.pos < self.buf.len() as i32 {
            let Position(x, y) = self.current_position();
            if x == self.window.get_max_x() - 1 {
                self.window.mv(y + 1, 0);
            } else {
                self.window.mv(y, x + 1);
            }
            self.pos += 1;
        }
    }

    fn move_to_start(&mut self) {
        let Position(x, y) = self.line_start_position();
        self.window.mv(y, x);
        self.pos = 0;
    }

    fn move_to_end(&mut self) {
        let Position(x, y) = self.line_end_position();
        self.window.mv(y, x);
        self.pos = self.buf.len() as i32;
    }

    fn line_start_position(&self) -> Position {
        let y = self.window.get_cur_y();
        let column = self.window.get_max_x();
        let line_count = (self.pos + 1 - (column - self.prompt.len() as i32) + column - 1) / column + 1;
        Position(self.prompt.len() as i32, y - line_count + 1)
    }

    fn line_end_position(&self) -> Position {
        let data_len = self.buf.len() as i32;
        let column = self.window.get_max_x();
        let Position(x, y) = self.line_start_position();
        if data_len <= column - self.prompt.len() as i32 {
            Position(x + data_len, y)
        } else {
            let line_count = (data_len - (column - self.prompt.len() as i32) + column - 1) / column + 1;
            let end_x = (data_len - (column - self.prompt.len() as i32)) % column;
            let end_y = y + line_count - 1;
            Position(end_x, end_y)
        }
    }

    #[allow(dead_code)]
    fn debug_print_buf(&self) {
        println!("\nbuf: {}, {}", String::from_utf8(self.buf.clone()).unwrap(), self.buf.len());
    }

    #[allow(dead_code)]
    fn debug_print_current_position(&self) {
        println!("\n{:?}", self.current_position());
    }

    #[allow(dead_code)]
    fn debug_print_pos(&self) {
        print!("\npos: {}", self.pos);
    }
}

