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

const CHUNKS: [u8; 18] = [
    0x43, 0x48, 0x4e, 0x4b, 0x00, 0x02, 0x02, 0x01, 0x03, 0x00, 0xaa, 0xbb, 0xcc, 0x02, 0x00, 0x00,
    0xff, 0xee,
];

/// Prints the README limit example: the offsets stay the file offsets and the last line is short.
#[test]
fn skip_four_limit_six_width_four_matches_worked_example() {
    let (code, out, err) = run_cli(
        &["dump", "--skip", "4", "--limit", "6", "--width", "4", "-"],
        &CHUNKS,
    );
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "00000004  00 02 02 01  |....|\n00000008  03 00        |..|\n"
    );
    assert_eq!(err, "");
}

/// A limit alone counts from the start; the equals form works like the separate form.
#[test]
fn limit_without_skip_counts_from_the_start() {
    let (code, out, err) = run_cli(&["dump", "--limit=4", "-"], &CHUNKS);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        format!("00000000  43 48 4e 4b{}|CHNK|\n", " ".repeat(38))
    );
    assert_eq!(err, "");
}

/// A limit that ends on a line boundary adds no empty line.
#[test]
fn limit_on_a_line_boundary_prints_whole_lines_only() {
    let (code, out, err) = run_cli(&["dump", "--width", "8", "--limit", "16", "-"], &CHUNKS);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "00000000  43 48 4e 4b 00 02 02 01  |CHNK....|\n00000008  03 00 aa bb cc 02 00 00  |........|\n"
    );
    assert_eq!(err, "");
}

/// Zero prints nothing; a limit larger than the rest of the data changes nothing.
#[test]
fn zero_and_oversized_limits() {
    assert_eq!(
        run_cli(&["dump", "--limit", "0", "-"], &CHUNKS),
        (0, String::new(), String::new())
    );
    let (code, all, _) = run_cli(&["dump", "-"], &CHUNKS);
    assert_eq!(code, 0);
    for limit in ["18", "19", "1000000000"] {
        let (code, out, err) = run_cli(&["dump", "--limit", limit, "-"], &CHUNKS);
        assert_eq!(
            (code, out, err),
            (0, all.clone(), String::new()),
            "limit {limit}"
        );
    }
    let (code, out, err) = run_cli(&["dump", "--skip", "18", "--limit", "5", "-"], &CHUNKS);
    assert_eq!((code, out, err), (0, String::new(), String::new()));
}

/// The last of a repeated option wins, as for --width and --skip.
#[test]
fn repeated_limit_uses_the_last_value() {
    let (code, out, _) = run_cli(
        &["dump", "--limit", "9", "--limit=2", "--width", "16", "-"],
        &CHUNKS,
    );
    assert_eq!(code, 0);
    assert_eq!(out, format!("00000000  43 48{}|CH|\n", " ".repeat(44)));
}

/// Invalid values and the wrong command are usage errors.
#[test]
fn invalid_limits_and_other_commands_are_usage_errors() {
    for (args, message) in [
        (
            vec!["dump", "--limit", "-"],
            "invalid --limit \"-\" (expected a whole number from 0 to 1000000000)",
        ),
        (
            vec!["dump", "--limit", "01", "-"],
            "invalid --limit \"01\" (expected a whole number from 0 to 1000000000)",
        ),
        (
            vec!["dump", "--limit=1000000001", "-"],
            "invalid --limit \"1000000001\" (expected a whole number from 0 to 1000000000)",
        ),
        (
            vec!["dump", "--limit=x", "-"],
            "invalid --limit \"x\" (expected a whole number from 0 to 1000000000)",
        ),
        (vec!["dump", "-", "--limit"], "--limit needs a value"),
        (
            vec!["decode", "a", "b", "--limit", "3"],
            "--limit is only valid for dump",
        ),
        (
            vec!["layout", "--limit=3", "a"],
            "--limit is only valid for dump",
        ),
    ] {
        let (code, out, err) = run_cli(&args, b"");
        assert_eq!(code, 2, "args {args:?}");
        assert_eq!(out, "", "args {args:?}");
        assert_eq!(err, format!("bytelens: {message}\n"), "args {args:?}");
    }
}
