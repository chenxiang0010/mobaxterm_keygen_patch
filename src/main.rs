use std::process;

use mobaxterm_keygen_patch::run;

fn main() {
    if let Err(e) = run() {
        eprintln!("Application error: {}", e);
        process::exit(1);
    };
}
