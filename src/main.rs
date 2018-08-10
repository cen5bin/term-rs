extern crate term_rs;
use term_rs::Terminal;

fn main() {
    Terminal::run(|command|{ format!("{}: command not found", command) } );
}
