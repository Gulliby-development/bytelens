use std::fs;
use std::path::PathBuf;

struct Out {
    code: i32,
    out: String,
    err: String,
}

const CHUNKS_SCHEMA: &str = "# a small container: a header, then records
struct Record
  u8 kind
  u16le size
  bytes[size] data
end

str[4] magic = \"CHNK\"
u16be version = 2
u8 count
Record[count] records
bytes[*] tail
";

const CHUNKS_HEX: &str = "43 48 4e 4b 00 02 02 01 03 00 aa bb cc 02 00 00 ff ee";

fn hex(text: &str) -> Vec<u8> {
    text.split_whitespace()
        .map(|p| u8::from_str_radix(p, 16).unwrap())
        .collect()
}

fn lines(items: &[&str]) -> String {
    items.iter().map(|l| format!("{l}\n")).collect()
}

fn work_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bytelens-{}-{}", std::process::id(), test));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn invoke(args: &[String], stdin: &[u8]) -> Out {
    let mut input = stdin;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = bytelens::run(args, &mut input, &mut out, &mut err);
    Out {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Runs `decode` on temp files (or stdin for the data) and checks the inputs stay unchanged.
fn decode_with(test: &str, schema: &str, data: &[u8], use_stdin: bool) -> (Out, String) {
    let dir = work_dir(test);
    let schema_path = dir.join("s.schema");
    let data_path = dir.join("d.bin");
    fs::write(&schema_path, schema).unwrap();
    fs::write(&data_path, data).unwrap();
    let schema_arg = schema_path.to_string_lossy().to_string();
    let (data_arg, stdin, display) = if use_stdin {
        ("-".to_string(), data, "<stdin>".to_string())
    } else {
        let text = data_path.to_string_lossy().to_string();
        let display = text.replace('\\', "/");
        (text, &[][..], display)
    };
    let args = vec!["decode".to_string(), schema_arg, data_arg];
    let result = invoke(&args, stdin);
    let schema_after = fs::read(&schema_path).unwrap();
    let data_after = fs::read(&data_path).unwrap();
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(schema_after, schema.as_bytes());
    assert_eq!(data_after, data);
    (result, display)
}

fn dec(test: &str, schema: &str, data: &[u8]) -> Out {
    decode_with(test, schema, data, true).0
}

fn assert_ok(result: &Out, expected: &[&str]) {
    assert_eq!(result.out, lines(expected));
    assert_eq!(result.err, "");
    assert_eq!(result.code, 0);
}

fn assert_fail(result: &Out, stdout: &[&str], stderr: &str) {
    assert_eq!(result.out, lines(stdout));
    assert_eq!(result.err, format!("bytelens: {stderr}\n"));
    assert_eq!(result.code, 1);
}

const CHUNKS_LINES: [&str; 12] = [
    "00000000  magic = \"CHNK\"",
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
];

#[test]
fn chunks_decodes_from_files() {
    let (result, _) = decode_with("chunks_files", CHUNKS_SCHEMA, &hex(CHUNKS_HEX), false);
    assert_ok(&result, &CHUNKS_LINES);
}

#[test]
fn chunks_decodes_from_stdin() {
    let result = dec("chunks_stdin", CHUNKS_SCHEMA, &hex(CHUNKS_HEX));
    assert_ok(&result, &CHUNKS_LINES);
}

#[test]
fn chunks_truncated_data() {
    let data = &hex(CHUNKS_HEX)[..11];
    let (result, display) = decode_with("trunc", CHUNKS_SCHEMA, data, false);
    assert_fail(
        &result,
        &CHUNKS_LINES[..6],
        &format!("{display}: offset 0x0000000a: need 3 bytes for \"records[0].data\", only 1 left"),
    );
}

#[test]
fn chunks_bad_magic() {
    let mut data = hex(CHUNKS_HEX);
    data[3] = b'X';
    let (result, display) = decode_with("badmagic", CHUNKS_SCHEMA, &data, false);
    assert_fail(
        &result,
        &[],
        &format!("{display}: offset 0x00000000: \"magic\" is \"CHNX\", expected \"CHNK\""),
    );
}

#[test]
fn integer_types_at_their_limits() {
    let cases: &[(&str, &str, &str)] = &[
        ("u8", "00", "0"),
        ("u8", "ff", "255"),
        ("i8", "80", "-128"),
        ("i8", "7f", "127"),
        ("i8", "ff", "-1"),
        ("u16le", "ff ff", "65535"),
        ("u16le", "01 02", "513"),
        ("u16be", "ff ff", "65535"),
        ("u16be", "01 02", "258"),
        ("i16le", "ff 7f", "32767"),
        ("i16le", "00 80", "-32768"),
        ("i16be", "7f ff", "32767"),
        ("i16be", "80 00", "-32768"),
        ("u32le", "ff ff ff ff", "4294967295"),
        ("u32le", "00 01 00 00", "256"),
        ("u32be", "ff ff ff ff", "4294967295"),
        ("u32be", "00 00 01 00", "256"),
        ("i32le", "ff ff ff 7f", "2147483647"),
        ("i32le", "00 00 00 80", "-2147483648"),
        ("i32be", "7f ff ff ff", "2147483647"),
        ("i32be", "80 00 00 00", "-2147483648"),
        ("u64le", "ff ff ff ff ff ff ff ff", "18446744073709551615"),
        ("u64le", "01 00 00 00 00 00 00 00", "1"),
        ("u64be", "ff ff ff ff ff ff ff ff", "18446744073709551615"),
        ("u64be", "00 00 00 00 00 00 00 01", "1"),
        ("i64le", "ff ff ff ff ff ff ff 7f", "9223372036854775807"),
        ("i64le", "00 00 00 00 00 00 00 80", "-9223372036854775808"),
        ("i64be", "7f ff ff ff ff ff ff ff", "9223372036854775807"),
        ("i64be", "80 00 00 00 00 00 00 00", "-9223372036854775808"),
    ];
    for (i, (ty, bytes, expected)) in cases.iter().enumerate() {
        let schema = format!("{ty} v\n");
        let result = dec(&format!("ints-{i}"), &schema, &hex(bytes));
        assert_ok(&result, &[&format!("00000000  v = {expected}")]);
    }
}

#[test]
fn bytes_display_rules() {
    let result = dec("bytes_empty", "bytes[0] b\n", &[]);
    assert_ok(&result, &["00000000  b = (empty)"]);
    let result = dec("bytes_three", "bytes[3] b\n", &hex("aa bb cc"));
    assert_ok(&result, &["00000000  b = aa bb cc"]);
    let all: Vec<u8> = (0..16).collect();
    let result = dec("bytes_sixteen", "bytes[16] b\n", &all);
    assert_ok(
        &result,
        &["00000000  b = 00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f"],
    );
    let all: Vec<u8> = (0..17).collect();
    let result = dec("bytes_seventeen", "bytes[17] b\n", &all);
    assert_ok(
        &result,
        &["00000000  b = 00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f ... (17 bytes)"],
    );
    let all: Vec<u8> = (0..20).collect();
    let result = dec("bytes_twenty", "bytes[20] b\n", &all);
    assert_ok(
        &result,
        &["00000000  b = 00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f ... (20 bytes)"],
    );
}

#[test]
fn str_strips_trailing_nuls_and_escapes_quote() {
    let result = dec("str_quote", "str[6] s\n", b"a\"b\0\0\0");
    assert_ok(&result, &[r#"00000000  s = "a\"b""#]);
    let result = dec("str_nul", "str[4] s\n", b"ab\0\0");
    assert_ok(&result, &[r#"00000000  s = "ab""#]);
}

#[test]
fn str_escapes_backslash_and_control_characters() {
    let result = dec("str_escape", "str[3] s\n", &hex("5c 01 7f"));
    assert_ok(&result, &[r#"00000000  s = "\\\x01\x7f""#]);
    let result = dec("str_escape_low", "str[2] s\n", &hex("1f 20"));
    assert_ok(&result, &[r#"00000000  s = "\x1f ""#]);
}

#[test]
fn str_keeps_non_ascii_and_empty_values() {
    let result = dec("str_utf8", "str[2] s\n", &hex("c3 a9"));
    assert_ok(&result, &["00000000  s = \"é\""]);
    let result = dec("str_all_nul", "str[3] s\n", &[0, 0, 0]);
    assert_ok(&result, &["00000000  s = \"\""]);
    let result = dec("str_zero", "str[0] s\n", &[]);
    assert_ok(&result, &["00000000  s = \"\""]);
}

#[test]
fn str_invalid_utf8_is_an_error() {
    let result = dec("str_invalid", "str[1] s\n", &[0xff]);
    assert_fail(
        &result,
        &[],
        "<stdin>: offset 0x00000000: invalid UTF-8 in \"s\"",
    );
    let result = dec("str_invalid_later", "u8 a\nstr[2] s\n", &hex("01 c3 28"));
    assert_fail(
        &result,
        &["00000000  a = 1"],
        "<stdin>: offset 0x00000001: invalid UTF-8 in \"s\"",
    );
}

#[test]
fn integer_array_with_number_count() {
    let all: Vec<u8> = (0..20).collect();
    let result = dec("array_twenty", "u8[20] v\n", &all);
    assert_ok(
        &result,
        &["00000000  v = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, ... (20 items)]"],
    );
    let all: Vec<u8> = (0..16).collect();
    let result = dec("array_sixteen", "u8[16] v\n", &all);
    assert_ok(
        &result,
        &["00000000  v = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]"],
    );
    let result = dec("array_zero", "u8[0] v\n", &[]);
    assert_ok(&result, &["00000000  v = []"]);
    let result = dec("array_wide", "i16be[3] v\n", &hex("00 01 ff fe 80 00"));
    assert_ok(&result, &["00000000  v = [1, -2, -32768]"]);
}

#[test]
fn integer_array_with_field_count() {
    let result = dec("array_field", "u16le n\nu8[n] items\n", &hex("02 00 05 06"));
    assert_ok(&result, &["00000000  n = 2", "00000002  items = [5, 6]"]);
    let result = dec("array_field_zero", "u8 n\nu16le[n] items\n", &hex("00"));
    assert_ok(&result, &["00000000  n = 0", "00000001  items = []"]);
}

#[test]
fn nested_structs_are_indented() {
    let schema = "struct In\n  u8 x\nend\nstruct Outer\n  u8 a\n  In inner\nend\nOuter o\n";
    let result = dec("nested", schema, &hex("01 02"));
    assert_ok(
        &result,
        &[
            "00000000  o",
            "00000000    a = 1",
            "00000001    inner",
            "00000001      x = 2",
        ],
    );
}

#[test]
fn struct_arrays_with_number_count() {
    let schema = "struct P\n  u8 x\nend\nP[2] ps\n";
    let result = dec("struct_array", schema, &hex("01 02"));
    assert_ok(
        &result,
        &[
            "00000000  ps[0]",
            "00000000    x = 1",
            "00000001  ps[1]",
            "00000001    x = 2",
        ],
    );
    let schema = "struct P\n  u8 x\nend\nP[0] ps\n";
    let result = dec("struct_array_zero", schema, &[]);
    assert_ok(&result, &["00000000  ps = []"]);
}

#[test]
fn star_arrays_of_integers() {
    let result = dec("star_int_empty", "i16be[*] xs\n", &[]);
    assert_ok(&result, &["00000000  xs = []"]);
    let result = dec("star_int", "i16be[*] xs\n", &hex("00 01 ff fe"));
    assert_ok(&result, &["00000000  xs = [1, -2]"]);
}

#[test]
fn star_array_of_integers_truncated() {
    let result = dec("star_int_trunc", "i16be[*] xs\n", &hex("ff fe 00 05 80"));
    assert_fail(
        &result,
        &[],
        "<stdin>: offset 0x00000004: need 2 bytes for \"xs[2]\", only 1 left",
    );
}

#[test]
fn star_arrays_of_bytes_and_structs() {
    let result = dec("star_bytes", "u8 a\nbytes[*] t\n", &hex("01 02 03"));
    assert_ok(&result, &["00000000  a = 1", "00000001  t = 02 03"]);
    let result = dec("star_bytes_empty", "bytes[*] t\n", &[]);
    assert_ok(&result, &["00000000  t = (empty)"]);
    let schema = "struct P\n  u8 x\nend\nP[*] es\n";
    let result = dec("star_struct", schema, &hex("01 02"));
    assert_ok(
        &result,
        &[
            "00000000  es[0]",
            "00000000    x = 1",
            "00000001  es[1]",
            "00000001    x = 2",
        ],
    );
    let result = dec("star_struct_empty", schema, &[]);
    assert_ok(&result, &["00000000  es = []"]);
}

#[test]
fn empty_star_struct_element_is_an_error() {
    let schema = "struct E\n  bytes[0] z\nend\nE[*] es\n";
    let result = dec("star_empty_element", schema, &hex("01"));
    assert_fail(
        &result,
        &["00000000  es[0]", "00000000    z = (empty)"],
        "<stdin>: offset 0x00000000: \"es[0]\" is empty",
    );
}

#[test]
fn truncated_integer() {
    let result = dec("trunc_int", "u16le a\n", &hex("01"));
    assert_fail(
        &result,
        &[],
        "<stdin>: offset 0x00000000: need 2 bytes for \"a\", only 1 left",
    );
}

#[test]
fn truncated_bytes_and_str() {
    let result = dec("trunc_bytes", "u8 a\nbytes[4] b\n", &hex("01 02 03"));
    assert_fail(
        &result,
        &["00000000  a = 1"],
        "<stdin>: offset 0x00000001: need 4 bytes for \"b\", only 2 left",
    );
    let result = dec("trunc_str", "str[4] s\n", &hex("41"));
    assert_fail(
        &result,
        &[],
        "<stdin>: offset 0x00000000: need 4 bytes for \"s\", only 1 left",
    );
}

#[test]
fn truncated_integer_array() {
    let result = dec("trunc_array", "u16le[3] v\n", &hex("01 00 02 00 03"));
    assert_fail(
        &result,
        &[],
        "<stdin>: offset 0x00000000: need 6 bytes for \"v\", only 5 left",
    );
}

#[test]
fn truncated_struct_and_struct_array_element() {
    let result = dec(
        "trunc_struct",
        "struct S\n  u16le x\nend\nS s\n",
        &hex("01"),
    );
    assert_fail(
        &result,
        &["00000000  s"],
        "<stdin>: offset 0x00000000: need 2 bytes for \"s.x\", only 1 left",
    );
    let result = dec(
        "trunc_struct_array",
        "struct S\n  u8 x\nend\nS[2] ss\n",
        &hex("01"),
    );
    assert_fail(
        &result,
        &["00000000  ss[0]", "00000000    x = 1", "00000001  ss[1]"],
        "<stdin>: offset 0x00000001: need 1 bytes for \"ss[1].x\", only 0 left",
    );
}

#[test]
fn count_larger_than_data() {
    let result = dec("count_big", "u16le n\nu8[n] items\n", &hex("ff ff 01 02"));
    assert_fail(
        &result,
        &["00000000  n = 65535"],
        "<stdin>: offset 0x00000002: need 65535 bytes for \"items\", only 2 left",
    );
}

#[test]
fn count_above_65535_is_rejected() {
    let result = dec("count_over", "u32le n\nu8[n] items\n", &hex("70 11 01 00"));
    assert_fail(
        &result,
        &["00000000  n = 70000"],
        "<stdin>: offset 0x00000004: count 70000 of \"items\" exceeds 65535",
    );
}

#[test]
fn trailing_bytes_are_reported() {
    let result = dec("trailing_two", "u8 a\n", &hex("01 02 03"));
    assert_fail(
        &result,
        &["00000000  a = 1"],
        "<stdin>: offset 0x00000001: 2 trailing bytes after the last field",
    );
    let result = dec("trailing_one", "u8 a\n", &hex("01 02"));
    assert_fail(
        &result,
        &["00000000  a = 1"],
        "<stdin>: offset 0x00000001: 1 trailing byte after the last field",
    );
}

#[test]
fn integer_literal_mismatch() {
    let result = dec("lit_int", "u8 a = 5\n", &hex("07"));
    assert_fail(
        &result,
        &[],
        "<stdin>: offset 0x00000000: \"a\" is 7, expected 5",
    );
    let result = dec("lit_neg", "i8 a = -5\n", &hex("00"));
    assert_fail(
        &result,
        &[],
        "<stdin>: offset 0x00000000: \"a\" is 0, expected -5",
    );
    let result = dec("lit_hex", "u16be a = 0x1F\n", &hex("00 20"));
    assert_fail(
        &result,
        &[],
        "<stdin>: offset 0x00000000: \"a\" is 32, expected 31",
    );
}

#[test]
fn matching_literals_are_printed() {
    let result = dec(
        "lit_ok",
        "u8 a = 5\nstr[2] s = \"AB\"\nbytes[2] b = \"ab\"\n",
        b"\x05ABab",
    );
    assert_ok(
        &result,
        &[
            "00000000  a = 5",
            "00000001  s = \"AB\"",
            "00000003  b = 61 62",
        ],
    );
}

#[test]
fn str_literal_mismatch_uses_decode_display() {
    let result = dec("lit_str_space", "str[3] s = \"A B\"\n", &hex("41 42 43"));
    assert_fail(
        &result,
        &[],
        "<stdin>: offset 0x00000000: \"s\" is \"ABC\", expected \"A B\"",
    );
    let result = dec("lit_str_utf8", "str[2] s = \"AB\"\n", &hex("c3 a9"));
    assert_fail(
        &result,
        &[],
        "<stdin>: offset 0x00000000: \"s\" is \"é\", expected \"AB\"",
    );
    let result = dec("lit_str_invalid", "str[2] s = \"AB\"\n", &hex("ff fe"));
    assert_fail(
        &result,
        &[],
        "<stdin>: offset 0x00000000: invalid UTF-8 in \"s\"",
    );
}

#[test]
fn bytes_literal_mismatch_uses_hex_pairs() {
    let result = dec("lit_bytes", "bytes[2] b = \"ab\"\n", &hex("61 63"));
    assert_fail(
        &result,
        &[],
        "<stdin>: offset 0x00000000: \"b\" is 61 63, expected 61 62",
    );
}

#[test]
fn literal_mismatch_in_nested_field_names_the_path() {
    let schema = "struct H\n  u8 v = 1\nend\nu8 a\nH[2] hs\n";
    let result = dec("lit_nested", schema, &hex("09 01 02"));
    assert_fail(
        &result,
        &[
            "00000000  a = 9",
            "00000001  hs[0]",
            "00000001    v = 1",
            "00000002  hs[1]",
        ],
        "<stdin>: offset 0x00000002: \"hs[1].v\" is 2, expected 1",
    );
}
