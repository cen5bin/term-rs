use pancurses::{Window, initscr, noecho, Input};
use super::command::CommandHistory;

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
struct Position(i32, i32);

pub struct Terminal<F> {
    prompt: String,
    window: Window,
    history: CommandHistory,
    buf: Vec<u8>,
    origin: Position,
    process: F,
}

impl<F> Terminal<F>
    where F: Fn(String) -> String {
    pub fn run(process: F) {
        let window = initscr();
        window.keypad(true);
        noecho();
        let mut t = Terminal {
            prompt: "debug> ".to_owned(),
            window,
            history: CommandHistory::default(),
            buf: Vec::new(),
            origin: Position(0, 0),
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
        self.origin = self.current_position();
        loop {
            if let Some(ch) = self.window.getch() {
                match ch {
                    Input::Character(c) => {
                        match c {
                            '\n' => {
                                let ret = String::from_utf8(self.buf.clone()).unwrap();
                                self.clear_line();
                                self.window.printw(format!("{}\n", ret));
                                self.history.add_command(ret.clone());
                                return ret;
                            }
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
                    Input::KeyUp => { self.prev_command(); }
                    Input::KeyDown => { self.next_command(); }
                    Input::KeyLeft => { self.move_left(); }
                    Input::KeyRight => { self.move_right(); }
                    _ => {}
                }
            }
        }
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
    }

    fn next_command(&mut self) {
        self.clear_line();
        if let Some(command) = self.history.next_command() {
            self.buf.extend(command.as_bytes());
            self.window.printw(command);
        }
    }

    fn insert(&mut self, text: String) {
        if self.current_position() == self.end_position() {
            self.buf.extend(text.as_bytes());
            self.window.printw(text);
        } else {
            let tmp = {
                let cur = self.current_len();

                let pre = &self.buf[0..cur as usize];
                let end = &self.buf[cur as usize..];
                let mut tmp = Vec::new();
                tmp.extend(pre);
                tmp.extend(text.as_bytes());
                tmp.extend(end);
                tmp
            };
            for _ in 0..text.len() {
                self.move_right();
            }
            let end_pos = self.current_position();
            self.clear_line();
            self.buf = tmp;
            self.window.printw(String::from_utf8(self.buf.clone()).unwrap());
            self.window.mv(end_pos.1, end_pos.0);

        }
    }

    fn clear_to_start(&mut self) {
        let len = self.current_len();
        let tmp = self.buf[len as usize..].to_owned();
        self.clear_line();
        self.buf = tmp;
        self.window.printw(String::from_utf8(self.buf.clone()).unwrap());
        let Position(x, y) = self.origin;
        self.window.mv(y, x);
    }

    fn backspace(&mut self) {
        if self.current_position() != self.origin {
            self.move_left();
            let current_len = self.current_len();
            self.buf.remove(current_len as usize);
            let p = self.current_position();
            let tmp = self.buf.clone();
            self.clear_line();
            self.buf = tmp;
            self.window.printw(String::from_utf8(self.buf.clone()).unwrap());
            self.window.mv(p.1, p.0);
        }
    }

    fn clear_line(&mut self) {
        let mut y = self.end_position().1;
        while y >= self.origin.1 {
            self.window.mv(y, 0);
            self.window.deleteln();
            y -= 1;
        }
        self.buf.clear();
        self.print_prompt();
        debug_assert_eq!(self.origin, self.current_position());
    }

    fn current_position(&self) -> Position {
        Position(self.window.get_cur_x(), self.window.get_cur_y())
    }

    fn move_left(&mut self) {
        if self.current_position() != self.origin {
            let Position(x, y) = self.current_position();
            if x == 0 {
                self.window.mv(y - 1, self.window.get_max_x() - 1);
            } else {
                self.window.mv(y, x - 1);
            }
        }
    }

    fn move_right(&self) {
        if self.current_position() != self.end_position() {
            let Position(x, y) = self.current_position();
            if x == self.window.get_max_x() - 1 {
                self.window.mv(y + 1, 0);
            } else {
                self.window.mv(y, x + 1);
            }
        }
    }

    fn end_position(&self) -> Position {
        let len = self.buf.len() as i32;
        let line = self.window.get_max_x();
        let first_line = line - self.prompt.len() as i32;
        if (self.buf.len() as i32 ) < first_line {
            Position(self.origin.0 + self.buf.len() as i32, self.origin.1)
        } else {
            let left = len - first_line;
            let delta_y = (left + line - 1) / line;
            let x = left % line;
            Position(x, self.origin.1 + delta_y)
        }
    }

    fn move_to_start(&self) {
        let Position(x, y) = self.origin;
        self.window.mv(y, x);
    }

    fn move_to_end(&self) {
        let Position(x, y) = self.end_position();
        self.window.mv(y, x);
    }

    fn current_len(&self) -> i32 {
        let Position(ori_x, ori_y) = self.origin;
        let Position(cur_x, cur_y) = self.current_position();
        if ori_y == cur_y {
            cur_x - ori_x
        } else {
            let line = self.window.get_max_x();
            (cur_y - ori_y - 1) * line + (line - self.prompt.len() as i32) + cur_x
        }
    }
}