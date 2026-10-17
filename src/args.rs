use crate::text::quote;

/// The three usage lines printed for `--help`.
pub const USAGE: [&str; 3] = [
    "usage: bytelens decode SCHEMA FILE",
    "       bytelens layout SCHEMA",
    "       bytelens dump [--width N] [--skip N] FILE",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Decode,
    Layout,
    Dump,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub command: Command,
    pub files: Vec<String>,
    pub width: usize,
    pub skip: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Help,
    Run(Parsed),
}

fn parse_width(text: &str) -> Option<usize> {
    if text.is_empty() || text.len() > 2 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if text.starts_with('0') {
        return None;
    }
    let value: usize = text.parse().ok()?;
    (1..=64).contains(&value).then_some(value)
}

/// Parses a skip count from zero through one billion without leading zeroes.
fn parse_skip(text: &str) -> Option<usize> {
    if text == "0" {
        return Some(0);
    }
    if text.is_empty()
        || text.len() > 10
        || text.starts_with('0')
        || !text.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let value: usize = text.parse().ok()?;
    (value <= 1_000_000_000).then_some(value)
}

/// Parses the arguments; the error is the message without the `bytelens: ` prefix.
pub fn parse_args(args: &[String]) -> Result<Action, String> {
    let Some(first) = args.first() else {
        return Err("missing command (decode, layout or dump)".to_string());
    };
    if args.len() == 1 && first == "--help" {
        return Ok(Action::Help);
    }
    let command = match first.as_str() {
        "decode" => Command::Decode,
        "layout" => Command::Layout,
        "dump" => Command::Dump,
        _ => return Err(format!("unknown command {}", quote(first))),
    };
    let mut width: Option<&str> = None;
    let mut skip: Option<&str> = None;
    let mut files: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--width" {
            let Some(value) = args.get(i + 1) else {
                return Err("--width needs a value".to_string());
            };
            width = Some(value);
            i += 2;
            continue;
        }
        if arg == "--skip" {
            let Some(value) = args.get(i + 1) else {
                return Err("--skip needs a value".to_string());
            };
            skip = Some(value);
            i += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--width=") {
            width = Some(value);
        } else if let Some(value) = arg.strip_prefix("--skip=") {
            skip = Some(value);
        } else if arg.starts_with('-') && arg != "-" {
            return Err(format!("unknown option {}", quote(arg)));
        } else {
            files.push(arg.clone());
        }
        i += 1;
    }
    let value = if let Some(text) = width {
        parse_width(text).ok_or_else(|| {
            format!(
                "invalid --width {} (expected a whole number from 1 to 64)",
                quote(text)
            )
        })?
    } else {
        16
    };
    let skip_value = if let Some(text) = skip {
        parse_skip(text).ok_or_else(|| {
            format!(
                "invalid --skip {} (expected a whole number from 0 to 1000000000)",
                quote(text)
            )
        })?
    } else {
        0
    };
    if width.is_some() && command != Command::Dump {
        return Err("--width is only valid for dump".to_string());
    }
    if skip.is_some() && command != Command::Dump {
        return Err("--skip is only valid for dump".to_string());
    }
    let needed = if command == Command::Decode { 2 } else { 1 };
    if files.len() < needed {
        return Err("missing argument".to_string());
    }
    if files.len() > needed {
        return Err("too many arguments".to_string());
    }
    if files.iter().filter(|f| f.as_str() == "-").count() > 1 {
        return Err("standard input given twice".to_string());
    }
    Ok(Action::Run(Parsed {
        command,
        files,
        width: value,
        skip: skip_value,
    }))
}
