use std::path::PathBuf;

fn run_cli(args: &[&str], stdin: &[u8]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let mut input: &[u8] = stdin;
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = bytelens::run(&args, &mut input, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn assert_usage_error(args: &[&str], message: &str) {
    let (code, out, err) = run_cli(args, b"");
    assert_eq!(code, 2, "args {args:?}");
    assert_eq!(out, "", "args {args:?}");
    assert_eq!(err, format!("bytelens: {message}\n"), "args {args:?}");
}

fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bytelens-{}-{}", std::process::id(), test));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn shown(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

const WIDTH_ERROR: &str = "(expected a whole number from 1 to 64)";

#[test]
fn help_alone_prints_usage() {
    let (code, out, err) = run_cli(&["--help"], b"");
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "usage: bytelens decode SCHEMA FILE\n       bytelens layout SCHEMA\n       bytelens dump [--width N] [--skip N] FILE\n"
    );
    assert_eq!(err, "");
}

#[test]
fn help_after_command_is_unknown_option() {
    assert_usage_error(&["decode", "--help"], "unknown option \"--help\"");
    assert_usage_error(&["dump", "--help"], "unknown option \"--help\"");
}

#[test]
fn help_with_more_arguments_is_unknown_command() {
    assert_usage_error(&["--help", "dump"], "unknown command \"--help\"");
}

#[test]
fn missing_command() {
    assert_usage_error(&[], "missing command (decode, layout or dump)");
}

#[test]
fn unknown_command() {
    assert_usage_error(&["foo"], "unknown command \"foo\"");
    assert_usage_error(&["foo", "--width"], "unknown command \"foo\"");
}

#[test]
fn unknown_command_quotes_unprintable() {
    assert_usage_error(&["fo o"], "unknown command \"fo?o\"");
    assert_usage_error(&["é"], "unknown command \"?\"");
    assert_usage_error(&[""], "unknown command \"\"");
}

#[test]
fn width_without_value() {
    assert_usage_error(&["dump", "--width"], "--width needs a value");
    assert_usage_error(&["dump", "f", "--width"], "--width needs a value");
}

#[test]
fn unknown_option_is_found_before_later_errors() {
    assert_usage_error(&["dump", "-x", "--width"], "unknown option \"-x\"");
    assert_usage_error(&["dump", "--width", "0", "-x"], "unknown option \"-x\"");
    assert_usage_error(&["dump", "--wid"], "unknown option \"--wid\"");
    assert_usage_error(&["dump", "-ü"], "unknown option \"-?\"");
}

#[test]
fn invalid_width_forms() {
    for bad in ["0", "65", "016", "+5", "-3", "", "abc", "100", "1.5", "00"] {
        let message = format!("invalid --width \"{bad}\" {WIDTH_ERROR}");
        assert_usage_error(&["dump", "--width", bad, "f"], &message);
        let joined = format!("--width={bad}");
        assert_usage_error(&["dump", &joined, "f"], &message);
    }
}

#[test]
fn width_value_starting_with_dash_is_the_value() {
    assert_usage_error(
        &["dump", "--width", "-3", "f"],
        "invalid --width \"-3\" (expected a whole number from 1 to 64)",
    );
}

#[test]
fn width_boundaries_are_accepted() {
    let (code, out, _) = run_cli(&["dump", "--width", "1", "-"], b"a");
    assert_eq!(code, 0);
    assert_eq!(out, "00000000  61  |a|\n");
    let (code, out, _) = run_cli(&["dump", "--width=64", "-"], b"a");
    assert_eq!(code, 0);
    assert_eq!(out, format!("00000000  {:<191}  |a|\n", "61"));
}

#[test]
fn last_width_wins() {
    let data = b"0123456789";
    let (code, out, err) = run_cli(&["dump", "--width", "4", "--width=8", "-"], data);
    assert_eq!(code, 0);
    assert_eq!(err, "");
    assert_eq!(out.lines().count(), 2);
    let (_, out, _) = run_cli(&["dump", "--width=8", "--width", "4", "-"], data);
    assert_eq!(out.lines().count(), 3);
}

#[test]
fn invalid_width_beats_width_only_for_dump() {
    assert_usage_error(
        &["layout", "--width", "0"],
        "invalid --width \"0\" (expected a whole number from 1 to 64)",
    );
    assert_usage_error(
        &["layout", "--width", "8", "s"],
        "--width is only valid for dump",
    );
    assert_usage_error(
        &["decode", "--width=8", "a", "b"],
        "--width is only valid for dump",
    );
}

#[test]
fn file_count_errors() {
    assert_usage_error(&["decode"], "missing argument");
    assert_usage_error(&["decode", "s"], "missing argument");
    assert_usage_error(&["layout"], "missing argument");
    assert_usage_error(&["dump"], "missing argument");
    assert_usage_error(&["dump", "--width", "8"], "missing argument");
    assert_usage_error(&["layout", "a", "b"], "too many arguments");
    assert_usage_error(&["dump", "a", "b"], "too many arguments");
    assert_usage_error(&["decode", "a", "b", "c"], "too many arguments");
    assert_usage_error(&["decode", "-", "-", "x"], "too many arguments");
}

#[test]
fn standard_input_given_twice() {
    assert_usage_error(&["decode", "-", "-"], "standard input given twice");
}

#[test]
fn stdin_is_named_in_messages() {
    let (code, out, err) = run_cli(&["layout", "-"], b"u8[n] x\n");
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err, "bytelens: <stdin>:1: unknown length field \"n\"\n");
}

#[test]
fn stdin_schema_and_stdin_data() {
    let (code, out, err) = run_cli(&["layout", "-"], b"u8 a\n");
    assert_eq!(code, 0);
    assert_eq!(err, "");
    assert_eq!(
        out,
        "root (1 byte)\nOFFSET  SIZE  TYPE  FIELD\n0       1     u8    a\n"
    );
    let (code, out, err) = run_cli(&["dump", "-"], b"abc");
    assert_eq!(code, 0);
    assert_eq!(err, "");
    assert_eq!(out, format!("00000000  {:<47}  |abc|\n", "61 62 63"));
}

#[test]
fn missing_file_shows_backslash_as_slash() {
    let (code, out, err) = run_cli(&["dump", "no-such-dir\\missing.bin"], b"");
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(
        err,
        "bytelens: no-such-dir/missing.bin: cannot read the file\n"
    );
    let (code, _, err) = run_cli(&["layout", "nope-schema.txt"], b"");
    assert_eq!(code, 2);
    assert_eq!(err, "bytelens: nope-schema.txt: cannot read the file\n");
}

#[test]
fn schema_that_is_not_utf8() {
    let (code, out, err) = run_cli(&["layout", "-"], &[0x75, 0x38, 0xff, 0xfe]);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err, "bytelens: <stdin>: input is not valid UTF-8\n");
}

#[test]
fn dump_accepts_non_utf8_data() {
    let (code, out, err) = run_cli(&["dump", "-"], &[0xff, 0xfe]);
    assert_eq!(code, 0);
    assert_eq!(err, "");
    assert_eq!(out, format!("00000000  {:<47}  |..|\n", "ff fe"));
}

#[test]
fn schema_is_read_before_data_file() {
    let (code, out, err) = run_cli(&["decode", "-", "no-such-data.bin"], b"u8[n] x\n");
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err, "bytelens: <stdin>:1: unknown length field \"n\"\n");
    let (code, _, err) = run_cli(&["decode", "no-schema.txt", "no-data.bin"], b"");
    assert_eq!(code, 2);
    assert_eq!(err, "bytelens: no-schema.txt: cannot read the file\n");
    let (code, _, err) = run_cli(&["decode", "-", "no-data.bin"], b"u8 a\n");
    assert_eq!(code, 2);
    assert_eq!(err, "bytelens: no-data.bin: cannot read the file\n");
}

#[test]
fn decode_schema_from_file_data_from_stdin() {
    let dir = scratch("schema_file_stdin_data");
    let schema = dir.join("s.schema");
    std::fs::write(&schema, "u8 a\r\n").unwrap();
    let path = schema.to_string_lossy().to_string();
    let (code, out, err) = run_cli(&["decode", &path, "-"], &[7]);
    assert_eq!(code, 0);
    assert_eq!(out, "00000000  a = 7\n");
    assert_eq!(err, "");
    let (code, out, err) = run_cli(&["decode", &path, "-"], &[1, 2, 3]);
    assert_eq!(code, 1);
    assert_eq!(out, "00000000  a = 1\n");
    assert_eq!(
        err,
        "bytelens: <stdin>: offset 0x00000001: 2 trailing bytes after the last field\n"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn decode_error_exits_1_and_names_the_file() {
    let dir = scratch("decode_exit_1");
    let data = dir.join("d.bin");
    std::fs::write(&data, [1u8, 2]).unwrap();
    let path = data.to_string_lossy().to_string();
    let (code, out, err) = run_cli(&["decode", "-", &path], b"u8 a\n");
    assert_eq!(code, 1);
    assert_eq!(out, "00000000  a = 1\n");
    assert_eq!(
        err,
        format!(
            "bytelens: {}: offset 0x00000001: 1 trailing byte after the last field\n",
            shown(&data)
        )
    );
    let (code, out, err) = run_cli(&["dump", &path], b"");
    assert_eq!(code, 0);
    assert_eq!(err, "");
    assert_eq!(out, format!("00000000  {:<47}  |..|\n", "01 02"));
    assert_eq!(std::fs::read(&data).unwrap(), vec![1u8, 2]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn schema_bom_and_crlf_are_accepted_through_run() {
    let (code, out, err) = run_cli(&["decode", "-", "-x"], b"");
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err, "bytelens: unknown option \"-x\"\n");
    let (code, out, _) = run_cli(&["layout", "-"], b"\xef\xbb\xbfu8 a\r\n");
    assert_eq!(code, 0);
    assert!(out.starts_with("root (1 byte)\n"));
}

#[test]
fn exit_codes_cover_0_1_and_2() {
    assert_eq!(run_cli(&["--help"], b"").0, 0);
    assert_eq!(run_cli(&["dump", "-"], b"").0, 0);
    assert_eq!(run_cli(&["layout", "-"], b"").0, 2);
    let dir = scratch("exit_codes");
    let data = dir.join("d.bin");
    std::fs::write(&data, [9u8]).unwrap();
    let path = data.to_string_lossy().to_string();
    assert_eq!(run_cli(&["decode", "-", &path], b"u8 a = 5\n").0, 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn empty_schema_is_no_fields() {
    let (code, out, err) = run_cli(&["layout", "-"], b"");
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err, "bytelens: <stdin>: no fields\n");
}

#[test]
fn no_output_contains_carriage_return() {
    let runs: Vec<(Vec<&str>, &[u8])> = vec![
        (vec!["--help"], b""),
        (vec!["foo"], b""),
        (vec!["dump", "-"], b"abc\r\n"),
        (vec!["layout", "-"], b"u8 a\r\nu16le b\r\n"),
        (vec!["layout", "-"], b"u8[n] x\r\n"),
        (vec!["dump", "--width", "0", "-"], b""),
    ];
    for (args, stdin) in runs {
        let (_, out, err) = run_cli(&args, stdin);
        assert!(!out.contains('\r'), "stdout of {args:?}");
        assert!(!err.contains('\r'), "stderr of {args:?}");
    }
    let dir = scratch("no_cr");
    let data = dir.join("d.bin");
    std::fs::write(&data, b"\r\n\r\n").unwrap();
    let path = data.to_string_lossy().to_string();
    let (_, out, err) = run_cli(&["decode", "-", &path], b"u8 a\r\n");
    assert!(!out.contains('\r') && !err.contains('\r'));
    std::fs::remove_dir_all(&dir).unwrap();
}
