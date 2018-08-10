#[derive(Default)]
pub struct CommandHistory {
    history: Vec<String>,
    cur: i32,
}

impl CommandHistory {
    pub fn default() -> Self {
        Default::default()
    }

    pub fn prev_command(&mut self) -> Option<&String> {
        if self.cur < 0 {
            None
        } else {
            self.cur -= 1;
            self.history.get(self.cur as usize)

        }
    }

    pub fn next_command(&mut self) -> Option<&String> {
        if self.cur == self.history.len() as i32 {
            None
        } else {
            self.cur += 1;
            let ret = self.history.get(self.cur as usize);
            ret
        }
    }

    pub fn add_command(&mut self, command: String) {
        self.history.push(command);
        self.cur = self.history.len() as i32;
    }

    pub fn at_top(&self) -> bool {
        self.history.len() as i32 == self.cur
    }
}