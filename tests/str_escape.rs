use std::fs;

/// Decodes `data` (given on standard input) with the one-field schema `str[len] s` and returns
/// the exit code, standard output and standard error.
fn decode_str(test: &str, data: &[u8]) -> (i32, String, String) {
    decode_with(test, &format!("str[{}] s\n", data.len()), data)
}

/// The same with a schema text of its own.
fn decode_with(test: &str, schema_text: &str, data: &[u8]) -> (i32, String, String) {
    let dir = std::env::temp_dir().join(format!("bytelens-{}-{}", std::process::id(), test));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let schema = dir.join("s.schema");
    fs::write(&schema, schema_text).unwrap();
    let args = vec![
        "decode".to_string(),
        schema.to_string_lossy().into_owned(),
        "-".to_string(),
    ];
    let mut input = data;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = bytelens::run(&args, &mut input, &mut out, &mut err);
    let _ = fs::remove_dir_all(&dir);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn assert_shown(test: &str, data: &[u8], shown: &str) {
    let (code, out, err) = decode_str(test, data);
    assert_eq!(err, "");
    assert_eq!(code, 0);
    assert_eq!(out, format!("00000000  s = {shown}\n"));
}

#[test]
fn c1_control_is_escaped() {
    assert_shown("c1_one", b"\xc2\x85", r#""<U+0085>""#);
}

#[test]
fn csi_between_letters_is_escaped() {
    assert_shown("c1_csi", b"a\xc2\x9b[2Jb", r#""a<U+009B>[2Jb""#);
}

#[test]
fn c1_range_ends_at_u009f() {
    let (code, out, err) = decode_str("c1_range", b"\xc2\x80\xc2\x9f\xc2\xa0\xc2\xa1");
    assert_eq!(err, "");
    assert_eq!(code, 0);
    assert_eq!(
        out.as_bytes(),
        b"00000000  s = \"<U+0080><U+009F>\xc2\xa0\xc2\xa1\"\n"
    );
}

#[test]
fn bidirectional_controls_are_escaped() {
    assert_shown(
        "bidi",
        b"a\xe2\x80\xaeb\xe2\x81\xa6c\xe2\x81\xa9d\xe2\x80\x8ee\xe2\x80\x8f",
        r#""a<U+202E>b<U+2066>c<U+2069>d<U+200E>e<U+200F>""#,
    );
}

#[test]
fn separators_and_byte_order_mark_are_escaped() {
    assert_shown(
        "separators",
        b"x\xe2\x80\xa8y\xe2\x80\xa9z\xef\xbb\xbf",
        r#""x<U+2028>y<U+2029>z<U+FEFF>""#,
    );
}

#[test]
fn neighbours_of_the_escaped_ranges_stay_as_written() {
    let data = b"\xe2\x80\x8b\xe2\x80\xaf\xe2\x81\xa5\xe2\x81\xaa\xef\xbf\xbe\xc3\xa9\xe4\xb8\xad";
    let (code, out, err) = decode_str("neighbours", data);
    assert_eq!(err, "");
    assert_eq!(code, 0);
    let mut want = b"00000000  s = \"".to_vec();
    want.extend_from_slice(data);
    want.extend_from_slice(b"\"\n");
    assert_eq!(out.as_bytes(), want.as_slice());
}

#[test]
fn escapes_mix_with_the_existing_ones() {
    assert_shown(
        "mixed",
        b"\x01\"\\\xc2\x85\x7f\xe2\x80\xaa",
        r#""\x01\"\\<U+0085>\x7f<U+202A>""#,
    );
}

#[test]
fn trailing_nuls_are_dropped_before_the_escape() {
    assert_shown("nuls", b"\xc2\x9b\0\0", r#""<U+009B>""#);
}

#[test]
fn invalid_utf8_is_still_an_error() {
    let (code, out, err) = decode_str("invalid", b"\xc2");
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(err.ends_with("invalid UTF-8 in \"s\"\n"), "{err}");
}

#[test]
fn a_failed_literal_shows_the_escape_too() {
    let (code, out, err) = decode_with("literal", "str[2] s = \"ab\"\n", b"\xc2\x85");
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(
        err.ends_with("offset 0x00000000: \"s\" is \"<U+0085>\", expected \"ab\"\n"),
        "{err}"
    );
}
