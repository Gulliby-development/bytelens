/// Runs the library entry point with in-memory stdin, stdout, and stderr buffers.
fn run_cli(args: &[&str], stdin: &[u8]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
    let mut input: &[u8] = stdin;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = bytelens::run(&args, &mut input, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// Asserts a usage error has the expected message and no stdout.
fn assert_usage_error(args: &[&str], message: &str) {
    let (code, out, err) = run_cli(args, b"");
    assert_eq!(code, 2, "args {args:?}");
    assert_eq!(out, "", "args {args:?}");
    assert_eq!(err, format!("bytelens: {message}\n"), "args {args:?}");
}

const CHUNKS: [u8; 18] = [
    0x43, 0x48, 0x4e, 0x4b, 0x00, 0x02, 0x02, 0x01, 0x03, 0x00, 0xaa, 0xbb, 0xcc, 0x02, 0x00, 0x00,
    0xff, 0xee,
];

/// Prints the README skip-six example with original, unaligned offsets.
#[test]
fn skip_six_width_four_matches_worked_example() {
    let (code, out, err) = run_cli(&["dump", "--skip", "6", "--width", "4", "-"], &CHUNKS);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "00000006  02 01 03 00  |....|\n0000000a  aa bb cc 02  |....|\n0000000e  00 00 ff ee  |....|\n"
    );
    assert_eq!(err, "");
}

/// Prints the README skip-four example with the default width.
#[test]
fn skip_four_matches_worked_example() {
    let (code, out, err) = run_cli(&["dump", "--skip", "4", "-"], &CHUNKS);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "00000004  00 02 02 01 03 00 aa bb cc 02 00 00 ff ee        |..............|\n"
    );
    assert_eq!(err, "");
}

/// Confirms omission and both explicit zero forms preserve the original dump API output.
#[test]
fn zero_skip_preserves_default_and_dump_api_behavior() {
    let expected = format!(
        "00000000  43 48 4e 4b 00 02 02 01 03 00 aa bb cc 02 00 00  |CHNK............|\n00000010  ff ee{}|..|\n",
        " ".repeat(44)
    );
    for args in [
        &["dump", "-"][..],
        &["dump", "--skip", "0", "-"][..],
        &["dump", "--skip=0", "-"][..],
    ] {
        let (code, out, err) = run_cli(args, &CHUNKS);
        assert_eq!(code, 0, "args {args:?}");
        assert_eq!(out, expected, "args {args:?}");
        assert_eq!(err, "", "args {args:?}");
    }

    let zero_width = [0x41, 0x42];
    let expected_zero_width = vec![
        "00000000  41  |A|".to_string(),
        "00000001  42  |B|".to_string(),
    ];
    assert_eq!(bytelens::dump::dump(&zero_width, 0), expected_zero_width);
    assert_eq!(
        bytelens::dump::dump_from(&zero_width, 0, 0),
        expected_zero_width
    );
    assert_eq!(
        bytelens::dump::dump_from(&zero_width, 0, 1),
        vec!["00000001  42  |B|".to_string()]
    );
}

/// Checks EOF, beyond-EOF, maximum skip, and empty-input behavior.
#[test]
fn skips_at_and_beyond_eof_and_empty_input_print_nothing() {
    for skip in ["18", "19", "4096", "1000000000"] {
        let (code, out, err) = run_cli(&["dump", "--skip", skip, "-"], &CHUNKS);
        assert_eq!(code, 0, "skip {skip}");
        assert_eq!(out, "", "skip {skip}");
        assert_eq!(err, "", "skip {skip}");
    }
    for skip in ["0", "1", "1000000000"] {
        let (code, out, err) = run_cli(&["dump", "--skip", skip, "-"], b"");
        assert_eq!(code, 0, "empty data, skip {skip}");
        assert_eq!(out, "", "empty data, skip {skip}");
        assert_eq!(err, "", "empty data, skip {skip}");
    }
}

/// Allows skip options around operands and makes the final repeated value win.
#[test]
fn option_placement_and_last_skip_wins() {
    let expected_skip_two = format!("00000002  {:<47}  |23456789|\n", "32 33 34 35 36 37 38 39");
    let expected_skip_six = format!("00000006  {:<47}  |6789|\n", "36 37 38 39");
    let cases: [(&[&str], &str); 3] = [
        (
            &["dump", "--skip", "1", "--skip=2", "-"],
            &expected_skip_two,
        ),
        (
            &["dump", "--skip=2", "-", "--skip", "6"],
            &expected_skip_six,
        ),
        (
            &["dump", "--skip", "x", "--skip=2", "-"],
            &expected_skip_two,
        ),
    ];
    for (args, expected) in cases {
        let (code, out, err) = run_cli(args, b"0123456789");
        assert_eq!(code, 0, "args {args:?}");
        assert_eq!(out, expected, "args {args:?}");
        assert_eq!(err, "", "args {args:?}");
    }
}

/// Rejects malformed skip values in both option syntaxes with quoted diagnostics.
#[test]
fn invalid_skip_forms_are_quoted_and_rejected() {
    let invalid = [
        ("", "\"\""),
        ("007", "\"007\""),
        ("1.5", "\"1.5\""),
        ("+1", "\"+1\""),
        ("-1", "\"-1\""),
        ("1000000001", "\"1000000001\""),
        ("10000000000", "\"10000000000\""),
        ("x", "\"x\""),
        ("١", "\"?\""),
        ("1 2", "\"1?2\""),
    ];
    for (value, quoted) in invalid {
        let message =
            format!("invalid --skip {quoted} (expected a whole number from 0 to 1000000000)");
        assert_usage_error(&["dump", "--skip", value, "-"], &message);
        let joined = format!("--skip={value}");
        assert_usage_error(&["dump", &joined, "-"], &message);
    }
}

/// Requires a value for a final skip and treats a dash-prefixed token as its value.
#[test]
fn missing_skip_value_and_dash_prefixed_value() {
    assert_usage_error(&["dump", "--skip"], "--skip needs a value");
    assert_usage_error(&["dump", "-", "--skip"], "--skip needs a value");
    assert_usage_error(
        &["dump", "--skip", "-1", "-"],
        "invalid --skip \"-1\" (expected a whole number from 0 to 1000000000)",
    );
}

/// Rejects a valid skip option for decode.
#[test]
fn decode_rejects_skip_option() {
    assert_usage_error(
        &["decode", "--skip", "1", "schema", "data"],
        "--skip is only valid for dump",
    );
    assert_usage_error(
        &["decode", "--skip=1", "schema", "data"],
        "--skip is only valid for dump",
    );
}

/// Rejects a valid skip option for layout.
#[test]
fn layout_rejects_skip_option() {
    assert_usage_error(
        &["layout", "--skip", "1", "schema"],
        "--skip is only valid for dump",
    );
    assert_usage_error(
        &["layout", "--skip=1", "schema"],
        "--skip is only valid for dump",
    );
}

/// Preserves option-validation order and the earlier scan and operand errors.
#[test]
fn option_error_order_and_scan_precedence() {
    assert_usage_error(
        &["layout", "--skip", "x", "--width", "0", "schema"],
        "invalid --width \"0\" (expected a whole number from 1 to 64)",
    );
    assert_usage_error(
        &["layout", "--skip", "x", "--width", "8", "schema"],
        "invalid --skip \"x\" (expected a whole number from 0 to 1000000000)",
    );
    assert_usage_error(
        &["layout", "--width", "8", "--skip", "1", "schema"],
        "--width is only valid for dump",
    );
    assert_usage_error(
        &["layout", "--skip", "1", "schema"],
        "--skip is only valid for dump",
    );
    assert_usage_error(
        &["layout", "--width", "0", "--skip", "x", "-x"],
        "unknown option \"-x\"",
    );
    assert_usage_error(
        &["layout", "--skip", "1", "schema", "extra"],
        "--skip is only valid for dump",
    );
    assert_usage_error(&["dump", "--skip", "1", "-", "extra"], "too many arguments");
}

/// Keeps carriage returns out of both successful output and error diagnostics.
#[test]
fn skip_output_and_diagnostics_have_no_carriage_return() {
    let (code, out, err) = run_cli(&["dump", "--skip", "6", "--width", "4", "-"], &CHUNKS);
    assert_eq!(code, 0);
    assert!(!out.contains('\r'));
    assert_eq!(err, "");
    assert!(!err.contains('\r'));

    let (code, out, err) = run_cli(&["dump", "--skip", "bad", "-"], &CHUNKS);
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert!(!out.contains('\r'));
    assert!(!err.contains('\r'));
}
