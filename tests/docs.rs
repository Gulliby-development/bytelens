use std::fs;

fn readme() -> String {
    fs::read_to_string("README.md")
        .expect("README.md must be readable")
        .replace("\r\n", "\n")
}

fn fenced(lines: &[&str]) -> String {
    format!("```\n{}\n```\n", lines.join("\n"))
}

fn assert_block(lines: &[&str]) {
    let text = readme();
    let block = fenced(lines);
    assert!(text.contains(&block), "README.md lacks block:\n{block}");
}

#[test]
fn readme_has_required_headings() {
    let text = readme();
    for heading in [
        "## Install",
        "## Usage",
        "## Schema file",
        "## Output",
        "## Exit codes",
        "## Development",
    ] {
        assert!(
            text.lines().any(|line| line == heading),
            "missing heading {heading}"
        );
    }
}

#[test]
fn readme_has_usage_lines() {
    assert_block(&[
        "usage: bytelens decode SCHEMA FILE",
        "       bytelens layout SCHEMA",
        "       bytelens dump [--width N] FILE",
    ]);
}

#[test]
fn readme_has_chunks_schema() {
    assert_block(&[
        "# a small container: a header, then records",
        "struct Record",
        "  u8 kind",
        "  u16le size",
        "  bytes[size] data",
        "end",
        "",
        r#"str[4] magic = "CHNK""#,
        "u16be version = 2",
        "u8 count",
        "Record[count] records",
        "bytes[*] tail",
    ]);
}

#[test]
fn readme_has_hex_block() {
    assert_block(&["43 48 4e 4b 00 02 02 01 03 00 aa bb cc 02 00 00 ff ee"]);
}

#[test]
fn readme_has_dump_output() {
    assert_block(&[
        "00000000  43 48 4e 4b 00 02 02 01 03 00 aa bb cc 02 00 00  |CHNK............|",
        "00000010  ff ee                                            |..|",
    ]);
}

#[test]
fn readme_has_decode_output() {
    assert_block(&[
        r#"00000000  magic = "CHNK""#,
        "00000004  version = 2",
        "00000006  count = 2",
        "00000007  records[0]",
        "00000007    kind = 1",
        "00000008    size = 3",
        "0000000a    data = aa bb cc",
        "0000000d  records[1]",
        "0000000d    kind = 2",
        "0000000e    size = 0",
        "00000010    data = (empty)",
        "00000010  tail = ff ee",
    ]);
}

#[test]
fn readme_has_layout_output() {
    assert_block(&[
        "root (variable size)",
        "OFFSET  SIZE  TYPE           FIELD",
        "0       4     str[4]         magic",
        "4       2     u16be          version",
        "6       1     u8             count",
        "7       ?     Record[count]  records",
        "?       ?     bytes[*]       tail",
        "",
        "struct Record (variable size)",
        "OFFSET  SIZE  TYPE         FIELD",
        "0       1     u8           kind",
        "1       2     u16le        size",
        "3       ?     bytes[size]  data",
    ]);
}

#[test]
fn readme_has_install_development_and_exit_codes() {
    let text = readme();
    assert!(text.contains("cargo build --release"));
    assert!(text.contains("target/release/bytelens"));
    assert_block(&[
        "cargo fmt --check",
        "cargo clippy -- -D warnings",
        "cargo test",
    ]);
    assert!(text.contains("- 0: success"));
    assert!(text.contains("- 1: the data does not fit the schema"));
    assert!(text.contains("- 2: usage or input error"));
}
