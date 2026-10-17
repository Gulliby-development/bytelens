use std::io::{Read, Write};

use crate::args::{Action, Command, USAGE, parse_args};
use crate::decode::decode;
use crate::dump::dump_from;
use crate::layout::layout;
use crate::parse::parse_schema;
use crate::schema::Schema;
use crate::text::display_path;

fn read_input(path: &str, stdin: &mut dyn Read) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    let result = if path == "-" {
        stdin.read_to_end(&mut data).map(|_| ())
    } else {
        std::fs::read(path).map(|bytes| data = bytes)
    };
    match result {
        Ok(()) => Ok(data),
        Err(_) => Err(format!("{}: cannot read the file", display_path(path))),
    }
}

fn read_schema(path: &str, stdin: &mut dyn Read) -> Result<Schema, String> {
    let bytes = read_input(path, stdin)?;
    let display = display_path(path);
    let Ok(text) = String::from_utf8(bytes) else {
        return Err(format!("{display}: input is not valid UTF-8"));
    };
    parse_schema(&display, &text)
}

fn execute(
    command: Command,
    files: &[String],
    width: usize,
    skip: usize,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<i32, String> {
    match command {
        Command::Decode => {
            let schema = read_schema(&files[0], stdin)?;
            let data = read_input(&files[1], stdin)?;
            let (lines, error) = decode(&schema, &data, &display_path(&files[1]));
            for line in lines {
                let _ = writeln!(stdout, "{line}");
            }
            match error {
                Some(message) => {
                    let _ = writeln!(stderr, "bytelens: {message}");
                    Ok(1)
                }
                None => Ok(0),
            }
        }
        Command::Layout => {
            let schema = read_schema(&files[0], stdin)?;
            for line in layout(&schema) {
                let _ = writeln!(stdout, "{line}");
            }
            Ok(0)
        }
        Command::Dump => {
            let data = read_input(&files[0], stdin)?;
            for line in dump_from(&data, width, skip) {
                let _ = writeln!(stdout, "{line}");
            }
            Ok(0)
        }
    }
}

pub fn run(
    args: &[String],
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let parsed = match parse_args(args) {
        Ok(Action::Help) => {
            for line in USAGE {
                let _ = writeln!(stdout, "{line}");
            }
            return 0;
        }
        Ok(Action::Run(parsed)) => parsed,
        Err(message) => {
            let _ = writeln!(stderr, "bytelens: {message}");
            return 2;
        }
    };
    match execute(
        parsed.command,
        &parsed.files,
        parsed.width,
        parsed.skip,
        stdin,
        stdout,
        stderr,
    ) {
        Ok(code) => code,
        Err(message) => {
            let _ = writeln!(stderr, "bytelens: {message}");
            2
        }
    }
}
