/// Shows a value in double quotes; each code point outside `!`..`~` becomes `?`.
pub fn quote(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        if ('!'..='~').contains(&c) {
            out.push(c);
        } else {
            out.push('?');
        }
    }
    out.push('"');
    out
}

/// Name of a file argument in messages: `-` is `<stdin>`, `\` is shown as `/`.
pub fn display_path(path: &str) -> String {
    if path == "-" {
        "<stdin>".to_string()
    } else {
        path.replace('\\', "/")
    }
}

/// Splits text into numbered lines (from 1). One leading BOM is skipped, lines end at LF
/// and one CR before the LF is removed.
pub fn split_lines(text: &str) -> Vec<(usize, &str)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    text.split('\n')
        .enumerate()
        .map(|(i, line)| (i + 1, line.strip_suffix('\r').unwrap_or(line)))
        .collect()
}

/// An offset as 8 lowercase hex digits.
pub fn hex8(offset: u64) -> String {
    format!("{offset:08x}")
}

/// Bytes as lowercase hex pairs separated by single spaces.
pub fn hex_pairs(bytes: &[u8]) -> String {
    let parts: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
    parts.join(" ")
}
