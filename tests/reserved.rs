use std::path::PathBuf;

const RESERVED_NAMES: [&str; 18] = [
    "struct", "end", "bytes", "str", "u8", "i8", "u16le", "u16be", "i16le", "i16be", "u32le",
    "u32be", "i32le", "i32be", "u64le", "u64be", "i64le", "i64be",
];

fn parse_error(text: &str) -> String {
    match bytelens::parse::parse_schema("s", text) {
        Ok(_) => panic!("expected a parse error for {text:?}"),
        Err(message) => message,
    }
}

fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn run(args: &[String]) -> (i32, String, String) {
    let mut stdin: &[u8] = &[];
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = bytelens::run(args, &mut stdin, &mut stdout, &mut stderr);
    (
        code,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bytelens-reserved-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn reserved_struct_names_are_rejected() {
    for name in RESERVED_NAMES {
        let schema = format!("struct {name}\nu8 value\nend\nu8 root");
        assert_eq!(
            parse_error(&schema),
            format!("s:1: invalid name \"{name}\""),
            "{name}"
        );
    }
}

#[test]
fn reserved_top_level_field_names_are_rejected() {
    for name in RESERVED_NAMES {
        assert_eq!(
            parse_error(&format!("u8 {name}")),
            format!("s:1: invalid name \"{name}\""),
            "{name}"
        );
    }
}

#[test]
fn reserved_fields_inside_structs_are_rejected() {
    for name in RESERVED_NAMES {
        let schema = format!("struct Item\nu8 {name}\nend\nu8 root");
        assert_eq!(
            parse_error(&schema),
            format!("s:2: invalid name \"{name}\""),
            "{name}"
        );
    }
}

#[test]
fn non_reserved_lookalike_names_remain_valid() {
    let lookalikes = ["U8", "Struct", "END", "bytes2", "str_", "struct1", "_end"];
    for name in lookalikes {
        let struct_schema = format!("struct {name}\nu8 inside\nend\n{name} value");
        assert!(
            bytelens::parse::parse_schema("s", &struct_schema).is_ok(),
            "struct name {name}"
        );

        let field_type = if name == "_end" { "bytes[2]" } else { "u8" };
        let field_schema = format!("{field_type} {name}");
        assert!(
            bytelens::parse::parse_schema("s", &field_schema).is_ok(),
            "field name {name}"
        );
    }
}

#[test]
fn existing_validation_error_precedence_is_preserved() {
    assert_eq!(
        parse_error("struct u8 extra"),
        "s:1: expected \"struct NAME\""
    );
    assert_eq!(
        parse_error("struct Item\nu8 value\nend\nstruct Item"),
        "s:4: duplicate struct \"Item\""
    );
    assert_eq!(parse_error("unknown end"), "s:1: unknown type \"unknown\"");
    assert_eq!(parse_error("bytes end"), "s:1: bytes and str need a length");
    assert_eq!(parse_error("u8[07] end"), "s:1: invalid count \"07\"");
    assert_eq!(
        parse_error("u8[missing] end"),
        "s:1: unknown length field \"missing\""
    );
    assert_eq!(
        parse_error("i8 n\nu8[n] end"),
        "s:2: length field \"n\" is not an unsigned integer"
    );
    assert_eq!(
        parse_error("u8 a\nu8 a\nu8 end"),
        "s:2: duplicate field \"a\" (first on line 1)"
    );
    assert_eq!(parse_error("u8 i64be = 300"), "s:1: invalid name \"i64be\"");
}

#[test]
fn reserved_count_reference_names_keep_unknown_field_errors() {
    assert_eq!(
        parse_error("u8[end] x"),
        "s:1: unknown length field \"end\""
    );
    assert_eq!(
        parse_error("u8 n\nu8[u8] x"),
        "s:2: unknown length field \"u8\""
    );
}

#[test]
fn run_reports_reserved_name_error_for_a_schema_file() {
    let dir = scratch("schema");
    let schema = dir.join("r1.schema");
    std::fs::write(&schema, "struct u8\nu8 a\nend\nu8 b").unwrap();
    let schema = schema.canonicalize().unwrap();
    let schema_arg = schema.to_string_lossy().into_owned();

    let args = vec!["layout".to_string(), schema_arg.clone()];
    let (code, stdout, stderr) = run(&args);

    assert_eq!(code, 2);
    assert_eq!(normalize_line_endings(&stdout), "");
    assert_eq!(
        normalize_line_endings(&stderr),
        format!(
            "bytelens: {}:1: invalid name \"u8\"\n",
            schema_arg.replace('\\', "/")
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn readme_documents_reserved_declaration_names() {
    let readme = std::fs::read_to_string("README.md").unwrap();
    let expected = "- Names match `[A-Za-z_][A-Za-z0-9_]*` and are not reserved: `struct`, `end`, `bytes`, `str` and the 14 integer type names (`u8` to `i64be`) are invalid as struct and field names.";
    assert!(readme.lines().any(|line| line == expected));
}
