use std::path::PathBuf;

fn run_dump(args: &[&str], stdin: &[u8]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
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

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("bytelens-{}-{}", std::process::id(), name));
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn write(&self, file: &str, data: &[u8]) -> String {
        let path = self.0.join(file);
        std::fs::write(&path, data).unwrap();
        path.to_str().unwrap().to_string()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn dump_file(name: &str, args: &[&str], data: &[u8]) -> (i32, String, String) {
    let dir = TempDir::new(name);
    let path = dir.write("input.bin", data);
    let mut all: Vec<&str> = vec!["dump"];
    all.extend_from_slice(args);
    all.push(&path);
    run_dump(&all, b"")
}

const CHUNKS: [u8; 18] = [
    0x43, 0x48, 0x4e, 0x4b, 0x00, 0x02, 0x02, 0x01, 0x03, 0x00, 0xaa, 0xbb, 0xcc, 0x02, 0x00, 0x00,
    0xff, 0xee,
];

#[test]
fn chunks_bin_two_lines() {
    let (code, out, err) = dump_file("dump_chunks", &[], &CHUNKS);
    let expected = format!(
        "00000000  43 48 4e 4b 00 02 02 01 03 00 aa bb cc 02 00 00  |CHNK............|\n\
         00000010  ff ee{}|..|\n",
        " ".repeat(44)
    );
    assert_eq!(out, expected);
    assert_eq!(err, "");
    assert_eq!(code, 0);
}

#[test]
fn empty_file_prints_nothing() {
    let (code, out, err) = dump_file("dump_empty", &[], b"");
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
}

#[test]
fn width_one() {
    let (code, out, _) = dump_file("dump_w1", &["--width", "1"], b"AB");
    assert_eq!(out, "00000000  41  |A|\n00000001  42  |B|\n");
    assert_eq!(code, 0);
}

#[test]
fn width_seven_pads_last_line() {
    let (code, out, _) = dump_file("dump_w7", &["--width", "7"], b"abcdefghij");
    let expected = format!(
        "00000000  61 62 63 64 65 66 67  |abcdefg|\n00000007  68 69 6a{}  |hij|\n",
        " ".repeat(12)
    );
    assert_eq!(out, expected);
    assert_eq!(code, 0);
}

#[test]
fn width_sixteen_exact_multiple_has_no_extra_line() {
    let data = [0x41u8; 32];
    let (_, out, _) = dump_file("dump_w16", &["--width=16"], &data);
    let hex = ["41"; 16].join(" ");
    let expected =
        format!("00000000  {hex}  |AAAAAAAAAAAAAAAA|\n00000010  {hex}  |AAAAAAAAAAAAAAAA|\n");
    assert_eq!(out, expected);
}

#[test]
fn width_sixty_four() {
    let data = [0x42u8; 70];
    let (_, out, _) = dump_file("dump_w64", &["--width", "64"], &data);
    let full = ["42"; 64].join(" ");
    let short = ["42"; 6].join(" ");
    let expected = format!(
        "00000000  {full}  |{}|\n00000040  {short}{}  |BBBBBB|\n",
        "B".repeat(64),
        " ".repeat(191 - 17)
    );
    assert_eq!(out, expected);
}

#[test]
fn printable_boundaries() {
    let (_, out, _) = dump_file("dump_print", &["--width", "4"], &[0x1f, 0x20, 0x7e, 0x7f]);
    assert_eq!(out, "00000000  1f 20 7e 7f  |. ~.|\n");
}

#[test]
fn non_utf8_data_succeeds() {
    let (code, out, err) = dump_file("dump_bin", &["--width", "4"], &[0xff, 0xfe, 0x80]);
    assert_eq!(out, format!("00000000  ff fe 80{}  |...|\n", " ".repeat(3)));
    assert_eq!(err, "");
    assert_eq!(code, 0);
}

#[test]
fn reads_stdin() {
    let (code, out, err) = run_dump(&["dump", "-"], b"hi\n");
    assert_eq!(
        out,
        format!("00000000  68 69 0a{}  |hi.|\n", " ".repeat(39))
    );
    assert_eq!(err, "");
    assert_eq!(code, 0);
}

#[test]
fn output_has_no_carriage_return() {
    let (_, out, _) = run_dump(&["dump", "-"], b"a\r\nb\r\n");
    assert!(!out.contains('\r'));
    assert!(out.ends_with("|\n"));
}

#[test]
fn missing_file_is_input_error() {
    let dir = TempDir::new("dump_missing");
    let path = dir.0.join("nope.bin");
    let path = path.to_str().unwrap().to_string();
    let (code, out, err) = run_dump(&["dump", &path], b"");
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert!(err.starts_with("bytelens: "));
    assert!(err.ends_with(": cannot read the file\n"));
}

#[test]
fn offsets_are_lowercase_hex() {
    let data = vec![0u8; 0x1b];
    let (_, out, _) = dump_file("dump_off", &["--width", "1"], &data);
    assert!(out.contains("\n0000001a  00  |.|\n"));
}
