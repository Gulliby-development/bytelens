use bytelens::parse::parse_schema;
use bytelens::schema::{Base, Count, IntType, Literal, Schema};

const CHUNKS: &str = "# a small container: a header, then records\nstruct Record\n  u8 kind\n  u16le size\n  bytes[size] data\nend\n\nstr[4] magic = \"CHNK\"\nu16be version = 2\nu8 count\nRecord[count] records\nbytes[*] tail\n";

fn ok(text: &str) -> Schema {
    match parse_schema("s", text) {
        Ok(schema) => schema,
        Err(message) => panic!("unexpected error: {message}"),
    }
}

fn err(text: &str) -> String {
    match parse_schema("s", text) {
        Ok(_) => panic!("expected an error for {text:?}"),
        Err(message) => message,
    }
}

fn run_stdin(args: &[&str], input: &[u8]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let mut stdin = input;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = bytelens::run(&args, &mut stdin, &mut stdout, &mut stderr);
    (
        code,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

#[test]
fn chunks_schema_parses() {
    let schema = ok(CHUNKS);
    assert_eq!(schema.root.len(), 5);
    assert_eq!(schema.structs.len(), 1);
    assert_eq!(schema.structs[0].name, "Record");
    assert_eq!(schema.structs[0].line, 2);
    assert_eq!(schema.structs[0].fields.len(), 3);
    assert_eq!(schema.root[3].base, Base::Struct(0));
    assert_eq!(
        schema.root[3].count,
        Some(Count::Field("count".to_string()))
    );
    assert_eq!(schema.root[3].type_text, "Record[count]");
    assert_eq!(schema.root[4].count, Some(Count::Rest));
    assert_eq!(
        schema.root[0].literal,
        Some(Literal::Text(b"CHNK".to_vec()))
    );
    assert_eq!(schema.root[1].literal, Some(Literal::Int(2)));
}

#[test]
fn bom_is_skipped() {
    assert_eq!(ok("\u{feff}u8 a\nu16le b\n"), ok("u8 a\nu16le b\n"));
}

#[test]
fn bom_does_not_change_line_numbers() {
    assert_eq!(err("\u{feff}u8 9a"), "s:1: invalid name \"9a\"");
}

#[test]
fn crlf_parses_like_lf() {
    assert_eq!(ok("u8 a\r\nu16le b\r\n"), ok("u8 a\nu16le b\n"));
}

#[test]
fn crlf_line_numbers_are_physical() {
    assert_eq!(
        err("u8 a\r\nu8 a\r\n"),
        "s:2: duplicate field \"a\" (first on line 1)"
    );
}

#[test]
fn spaces_and_tabs_are_trimmed() {
    assert_eq!(ok(" \t u8 \t a \t\n\t\tu16le  b\n"), ok("u8 a\nu16le b\n"));
}

#[test]
fn comments_and_blank_lines_count_as_lines() {
    assert_eq!(
        err("# comment\n\n   \n  # more\nu8 9a\n"),
        "s:5: invalid name \"9a\""
    );
    assert_eq!(err("# c\n\n\nu8 9a"), "s:4: invalid name \"9a\"");
}

#[test]
fn struct_nested() {
    assert_eq!(
        err("struct A\nstruct B\n"),
        "s:2: struct definitions cannot be nested"
    );
}

#[test]
fn struct_needs_exactly_a_name() {
    assert_eq!(err("struct"), "s:1: expected \"struct NAME\"");
    assert_eq!(err("struct A B"), "s:1: expected \"struct NAME\"");
}

#[test]
fn struct_invalid_name() {
    assert_eq!(err("struct 1A"), "s:1: invalid name \"1A\"");
    assert_eq!(err("struct a-b"), "s:1: invalid name \"a-b\"");
}

#[test]
fn struct_duplicate() {
    assert_eq!(
        err("struct A\nu8 a\nend\nstruct A\n"),
        "s:4: duplicate struct \"A\""
    );
}

#[test]
fn end_errors() {
    assert_eq!(err("end x"), "s:1: expected \"end\"");
    assert_eq!(err("end"), "s:1: \"end\" without struct");
    assert_eq!(err("struct A\nend"), "s:2: struct \"A\" has no fields");
}

#[test]
fn two_problems_report_the_earlier_check() {
    assert_eq!(
        err("struct A\nstruct"),
        "s:2: struct definitions cannot be nested"
    );
    assert_eq!(err("struct 1A B"), "s:1: expected \"struct NAME\"");
    assert_eq!(err("end x"), "s:1: expected \"end\"");
    assert_eq!(err("bytes 9a"), "s:1: bytes and str need a length");
    assert_eq!(err("foo 9a"), "s:1: unknown type \"foo\"");
    assert_eq!(err("u8[n] 9a"), "s:1: unknown length field \"n\"");
    assert_eq!(err("u8 9a = 256"), "s:1: invalid name \"9a\"");
    assert_eq!(
        err("u8 a\nu8 a = 256"),
        "s:2: duplicate field \"a\" (first on line 1)"
    );
}

#[test]
fn star_must_be_last_in_root() {
    assert_eq!(
        err("u8[*] a\nu8 b"),
        "s:1: \"*\" is only allowed on the last field of the file layout"
    );
    assert_eq!(
        err("u8[*] a\nu8"),
        "s:1: \"*\" is only allowed on the last field of the file layout"
    );
    assert_eq!(ok("u8 b\nu8[*] a").root.len(), 2);
}

#[test]
fn star_inside_struct_is_rejected() {
    assert_eq!(
        err("struct A\nu8[*] a\nend"),
        "s:2: \"*\" is only allowed on the last field of the file layout"
    );
}

#[test]
fn field_needs_type_and_name() {
    let message = "s:1: expected \"TYPE NAME [= LITERAL]\"";
    assert_eq!(err("u8"), message);
    assert_eq!(err("u8 a 5"), message);
    assert_eq!(err("u8 a ="), message);
    assert_eq!(err("u8 a=1"), message);
}

#[test]
fn unknown_types() {
    assert_eq!(err("foo x"), "s:1: unknown type \"foo\"");
    assert_eq!(err("u8(3) x"), "s:1: unknown type \"u8(3)\"");
    assert_eq!(err("u8[3 x"), "s:1: unknown type \"u8[3\"");
    assert_eq!(err("U8 x"), "s:1: unknown type \"U8\"");
}

#[test]
fn struct_types_must_be_defined_earlier() {
    assert_eq!(err("struct B\nA a\nend\n"), "s:2: unknown type \"A\"");
    assert_eq!(err("struct A\nA a\nend\n"), "s:2: unknown type \"A\"");
    let schema = ok("struct A\nu8 x\nend\nstruct B\nA a\nend\nB b");
    assert_eq!(schema.structs[1].fields[0].base, Base::Struct(0));
    assert_eq!(schema.root[0].base, Base::Struct(1));
}

#[test]
fn bytes_and_str_need_a_length() {
    assert_eq!(err("bytes x"), "s:1: bytes and str need a length");
    assert_eq!(err("str x"), "s:1: bytes and str need a length");
}

#[test]
fn invalid_counts() {
    assert_eq!(err("u8[07] x"), "s:1: invalid count \"07\"");
    assert_eq!(err("u8[65536] x"), "s:1: invalid count \"65536\"");
    assert_eq!(err("u8[-1] x"), "s:1: invalid count \"-1\"");
    assert_eq!(err("u8[] x"), "s:1: invalid count \"\"");
    assert_eq!(err("u8[a-b] x"), "s:1: invalid count \"a-b\"");
}

#[test]
fn counts_at_their_limits() {
    let schema = ok("u8[0] a\nu8[65535] b\nu8[10] c");
    assert_eq!(schema.root[0].count, Some(Count::Fixed(0)));
    assert_eq!(schema.root[1].count, Some(Count::Fixed(65535)));
    assert_eq!(schema.root[2].count, Some(Count::Fixed(10)));
}

#[test]
fn length_fields() {
    assert_eq!(err("u8[n] x"), "s:1: unknown length field \"n\"");
    assert_eq!(err("u8[n] x\nu8 n"), "s:1: unknown length field \"n\"");
    assert_eq!(
        err("i8 n\nu8[n] x"),
        "s:2: length field \"n\" is not an unsigned integer"
    );
    assert_eq!(
        err("u8[2] n\nu8[n] x"),
        "s:2: length field \"n\" is not an unsigned integer"
    );
    assert_eq!(
        err("bytes[2] n\nu8[n] x"),
        "s:2: length field \"n\" is not an unsigned integer"
    );
    assert_eq!(
        ok("u32be n\nu8[n] x").root[1].count,
        Some(Count::Field("n".to_string()))
    );
}

#[test]
fn length_fields_are_scoped() {
    assert_eq!(
        err("u8 n\nstruct S\nu8[n] x\nend"),
        "s:3: unknown length field \"n\""
    );
    let schema = ok("struct S\nu8 n\nu8[n] x\nend\nu8 a");
    assert_eq!(
        schema.structs[0].fields[1].count,
        Some(Count::Field("n".to_string()))
    );
}

#[test]
fn invalid_and_duplicate_field_names() {
    assert_eq!(err("u8 9a"), "s:1: invalid name \"9a\"");
    assert_eq!(err("u8 a-b"), "s:1: invalid name \"a-b\"");
    assert_eq!(
        err("u8 a\nu8 a"),
        "s:2: duplicate field \"a\" (first on line 1)"
    );
    assert_eq!(
        err("# c\nu8 a\n\nu16le a"),
        "s:4: duplicate field \"a\" (first on line 2)"
    );
    assert_eq!(ok("u8 _a1\nu8 A").root.len(), 2);
    let schema = ok("struct S\nu8 a\nend\nu8 a");
    assert_eq!(schema.root[0].name, "a");
}

#[test]
fn accepted_literals() {
    let schema = ok(
        "u8 a = 255\ni8 b = -128\nu64le c = 0xFFFFFFFFFFFFFFFF\nstr[4] m = \"CHNK\"\nbytes[2] d = \"ab\"\nu8 e = 0\nu8 f = 0x1F\ni8 g = 127\nstr[3] h = \"A B\"\nbytes[0] z = \"\"",
    );
    let literals: Vec<_> = schema.root.iter().map(|f| f.literal.clone()).collect();
    assert_eq!(literals[0], Some(Literal::Int(255)));
    assert_eq!(literals[1], Some(Literal::Int(-128)));
    assert_eq!(literals[2], Some(Literal::Int(u64::MAX as i128)));
    assert_eq!(literals[3], Some(Literal::Text(b"CHNK".to_vec())));
    assert_eq!(literals[4], Some(Literal::Text(b"ab".to_vec())));
    assert_eq!(literals[5], Some(Literal::Int(0)));
    assert_eq!(literals[6], Some(Literal::Int(31)));
    assert_eq!(literals[7], Some(Literal::Int(127)));
    assert_eq!(literals[8], Some(Literal::Text(b"A B".to_vec())));
    assert_eq!(literals[9], Some(Literal::Text(Vec::new())));
    assert_eq!(schema.root[0].base, Base::Int(IntType::U8));
}

#[test]
fn literal_on_wrong_field() {
    let message = "a literal needs a number or a fixed-length bytes or str field";
    assert_eq!(err("u8[2] a = 1"), format!("s:1: {message}"));
    assert_eq!(err("u8 n\nbytes[n] b = \"x\""), format!("s:2: {message}"));
    assert_eq!(
        err("struct P\nu8 x\nend\nP p = 1"),
        format!("s:4: {message}")
    );
    assert_eq!(err("u8 n\nstr[n] s = \"x\""), format!("s:2: {message}"));
}

#[test]
fn invalid_literals() {
    assert_eq!(err("u8 a = 012"), "s:1: invalid literal \"012\"");
    assert_eq!(err("u8 a = 0x"), "s:1: invalid literal \"0x\"");
    assert_eq!(
        err("u8 a = 0x11111111111111111"),
        "s:1: invalid literal \"0x11111111111111111\""
    );
    assert_eq!(err("u8 a = abc"), "s:1: invalid literal \"abc\"");
    assert_eq!(err("u8 a = -"), "s:1: invalid literal \"-\"");
    assert_eq!(err("u8 a = 0xZZ"), "s:1: invalid literal \"0xZZ\"");
    assert_eq!(err("str[1] s = 1"), "s:1: invalid literal \"1\"");
    assert_eq!(err("str[1] s = \"\\\""), r#"s:1: invalid literal ""\"""#);
    assert_eq!(err("str[1] s = \"\"\""), r#"s:1: invalid literal """"""#);
}

#[test]
fn literal_value_out_of_range() {
    assert_eq!(err("u8 a = 256"), "s:1: value 256 does not fit u8");
    assert_eq!(err("i8 a = -129"), "s:1: value -129 does not fit i8");
    assert_eq!(err("i8 a = 128"), "s:1: value 128 does not fit i8");
    assert_eq!(err("u8 a = -1"), "s:1: value -1 does not fit u8");
    assert_eq!(
        err("u64le a = 18446744073709551616"),
        "s:1: value 18446744073709551616 does not fit u64le"
    );
    assert_eq!(
        err("u16be a = 0x10000"),
        "s:1: value 65536 does not fit u16be"
    );
    assert_eq!(
        err("u8 a = 999999999999999999999999999999999999999999"),
        "s:1: value 999999999999999999999999999999999999999999 does not fit u8"
    );
}

#[test]
fn string_literal_length_must_match() {
    assert_eq!(
        err("str[4] m = \"ABC\""),
        "s:1: string has 3 bytes but the field has 4"
    );
    assert_eq!(
        err("bytes[1] m = \"AB\""),
        "s:1: string has 2 bytes but the field has 1"
    );
    assert_eq!(
        err("bytes[2] m = \"\""),
        "s:1: string has 0 bytes but the field has 2"
    );
}

#[test]
fn struct_not_closed() {
    assert_eq!(
        err("u8 a\nstruct S\nu8 b"),
        "s:2: struct \"S\" is not closed"
    );
    assert_eq!(err("struct S"), "s:1: struct \"S\" is not closed");
}

#[test]
fn no_fields() {
    assert_eq!(err(""), "s: no fields");
    assert_eq!(err("# only a comment\n\n"), "s: no fields");
    assert_eq!(err("struct S\nu8 a\nend\n"), "s: no fields");
}

#[test]
fn error_uses_the_given_display_name() {
    assert_eq!(
        parse_schema("dir/x.schema", "u8 9a").unwrap_err(),
        "dir/x.schema:1: invalid name \"9a\""
    );
    assert_eq!(
        parse_schema("<stdin>", "").unwrap_err(),
        "<stdin>: no fields"
    );
}

#[test]
fn run_reports_schema_error_with_exit_2() {
    let (code, stdout, stderr) = run_stdin(&["layout", "-"], b"u8[n] x");
    assert_eq!(code, 2);
    assert_eq!(stdout, "");
    assert_eq!(stderr, "bytelens: <stdin>:1: unknown length field \"n\"\n");
}

#[test]
fn run_reports_no_fields_and_invalid_utf8() {
    let (code, stdout, stderr) = run_stdin(&["layout", "-"], b"");
    assert_eq!((code, stdout.as_str()), (2, ""));
    assert_eq!(stderr, "bytelens: <stdin>: no fields\n");
    let (code, stdout, stderr) = run_stdin(&["layout", "-"], &[0x75, 0xff]);
    assert_eq!((code, stdout.as_str()), (2, ""));
    assert_eq!(stderr, "bytelens: <stdin>: input is not valid UTF-8\n");
}

#[test]
fn run_accepts_bom_and_crlf_schema() {
    let (code, stdout, stderr) = run_stdin(&["layout", "-"], b"\xef\xbb\xbfu8 a\r\n");
    assert_eq!(code, 0);
    assert!(stdout.starts_with("root (1 byte)\n"));
    assert!(!stdout.contains('\r'));
    assert_eq!(stderr, "");
}
