use bytelens::run;

fn layout(schema: &str) -> (i32, String, String) {
    let args = vec!["layout".to_string(), "-".to_string()];
    let mut input: &[u8] = schema.as_bytes();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(&args, &mut input, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn lines(schema: &str) -> Vec<String> {
    let (code, out, err) = layout(schema);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(err, "");
    out.lines().map(str::to_string).collect()
}

fn cols(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

const CHUNKS: &str = "# a small container: a header, then records\nstruct Record\n  u8 kind\n  u16le size\n  bytes[size] data\nend\n\nstr[4] magic = \"CHNK\"\nu16be version = 2\nu8 count\nRecord[count] records\nbytes[*] tail\n";

const CHUNKS_LAYOUT: &str = "root (variable size)\n\
OFFSET  SIZE  TYPE           FIELD\n\
0       4     str[4]         magic\n\
4       2     u16be          version\n\
6       1     u8             count\n\
7       ?     Record[count]  records\n\
?       ?     bytes[*]       tail\n\
\n\
struct Record (variable size)\n\
OFFSET  SIZE  TYPE         FIELD\n\
0       1     u8           kind\n\
1       2     u16le        size\n\
3       ?     bytes[size]  data\n";

#[test]
fn chunks_layout_is_exact() {
    let (code, out, err) = layout(CHUNKS);
    assert_eq!(code, 0);
    assert_eq!(err, "");
    assert_eq!(out, CHUNKS_LAYOUT);
    assert_eq!(out.lines().count(), 13);
}

#[test]
fn chunks_layout_has_no_trailing_spaces_or_cr() {
    let (_, out, _) = layout(CHUNKS);
    assert!(!out.contains('\r'));
    for line in out.lines() {
        assert_eq!(line, line.trim_end(), "trailing space in {line:?}");
    }
}

#[test]
fn single_byte_root() {
    let (code, out, _) = layout("u8 a\n");
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "root (1 byte)\nOFFSET  SIZE  TYPE  FIELD\n0       1     u8    a\n"
    );
}

#[test]
fn single_byte_struct_heading() {
    let l = lines("struct S\nu8 a\nend\nS s\n");
    assert_eq!(l[0], "root (1 byte)");
    assert!(l.contains(&"struct S (1 byte)".to_string()));
}

#[test]
fn zero_length_root_is_zero_bytes() {
    let l = lines("u8[0] z\n");
    assert_eq!(l[0], "root (0 bytes)");
    assert_eq!(cols(&l[2]), ["0", "0", "u8[0]", "z"]);
}

#[test]
fn known_struct_array_exact() {
    let (code, out, _) = layout("struct P\nu16le x\nu16le y\nend\nP[3] ps\nu32be z\n");
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "root (16 bytes)\n\
OFFSET  SIZE  TYPE   FIELD\n\
0       12    P[3]   ps\n\
12      4     u32be  z\n\
\n\
struct P (4 bytes)\n\
OFFSET  SIZE  TYPE   FIELD\n\
0       2     u16le  x\n\
2       2     u16le  y\n"
    );
}

#[test]
fn all_integer_sizes() {
    let l = lines("u8 a\ni16be b\nu32le c\ni64be d\nu16le e\ni8 f\ni32be g\nu64le h\n");
    assert_eq!(l[0], "root (30 bytes)");
    let expect = [
        ["0", "1"],
        ["1", "2"],
        ["3", "4"],
        ["7", "8"],
        ["15", "2"],
        ["17", "1"],
        ["18", "4"],
        ["22", "8"],
    ];
    for (i, pair) in expect.iter().enumerate() {
        let c = cols(&l[2 + i]);
        assert_eq!(&c[..2], pair);
    }
}

#[test]
fn nested_known_sizes_are_summed() {
    let l = lines("struct A\nu8 a\nu16le b\nend\nstruct B\nA[2] x\nu8 y\nend\nB[2] b\nu8 t\n");
    assert_eq!(l.len(), 14);
    assert_eq!(l[0], "root (15 bytes)");
    assert_eq!(cols(&l[2]), ["0", "14", "B[2]", "b"]);
    assert_eq!(cols(&l[3]), ["14", "1", "u8", "t"]);
    assert_eq!(l[4], "");
    assert_eq!(l[5], "struct A (3 bytes)");
    assert_eq!(l[9], "");
    assert_eq!(l[10], "struct B (7 bytes)");
    assert_eq!(cols(&l[12]), ["0", "6", "A[2]", "x"]);
    assert_eq!(cols(&l[13]), ["6", "1", "u8", "y"]);
}

#[test]
fn structs_are_listed_in_definition_order() {
    let l = lines("struct Z\nu8 a\nend\nstruct B\nu8 a\nend\nu8 r\n");
    let headings: Vec<&String> = l.iter().filter(|s| s.contains(" (")).collect();
    assert_eq!(
        headings,
        ["root (1 byte)", "struct Z (1 byte)", "struct B (1 byte)"]
    );
}

#[test]
fn field_count_makes_size_and_later_offsets_unknown() {
    let l = lines("u8 n\nu8[n] a\nu16le b\nu8 c\n");
    assert_eq!(l[0], "root (variable size)");
    assert_eq!(cols(&l[2]), ["0", "1", "u8", "n"]);
    assert_eq!(cols(&l[3]), ["1", "?", "u8[n]", "a"]);
    assert_eq!(cols(&l[4]), ["?", "2", "u16le", "b"]);
    assert_eq!(cols(&l[5]), ["?", "1", "u8", "c"]);
}

#[test]
fn struct_with_unknown_size_makes_field_unknown() {
    let l = lines("struct S\nu8 n\nu8[n] d\nend\nS s\nu8 after\n");
    assert_eq!(l[0], "root (variable size)");
    assert_eq!(cols(&l[2]), ["0", "?", "S", "s"]);
    assert_eq!(cols(&l[3]), ["?", "1", "u8", "after"]);
    assert!(l.contains(&"struct S (variable size)".to_string()));
}

const LIMIT_STRUCTS: &str =
    "struct A\nbytes[32768] data\nend\nstruct B\nA[32768] a\nend\nstruct C\nB[1024] b\nend\n";

#[test]
fn size_of_exactly_two_to_the_40_is_shown() {
    let l = lines(&format!("{LIMIT_STRUCTS}C c\n"));
    assert_eq!(l[0], "root (1099511627776 bytes)");
    assert_eq!(cols(&l[2]), ["0", "1099511627776", "C", "c"]);
    assert!(l.contains(&"struct A (32768 bytes)".to_string()));
    assert!(l.contains(&"struct B (1073741824 bytes)".to_string()));
    assert!(l.contains(&"struct C (1099511627776 bytes)".to_string()));
}

#[test]
fn offset_of_exactly_two_to_the_40_is_shown_and_beyond_is_unknown() {
    let l = lines(&format!("{LIMIT_STRUCTS}C c\nu8 d\nu8 e\n"));
    assert_eq!(l[0], "root (variable size)");
    assert_eq!(cols(&l[2]), ["0", "1099511627776", "C", "c"]);
    assert_eq!(cols(&l[3]), ["1099511627776", "1", "u8", "d"]);
    assert_eq!(cols(&l[4]), ["?", "1", "u8", "e"]);
}

#[test]
fn size_above_two_to_the_40_is_unknown() {
    let l = lines(&format!(
        "{LIMIT_STRUCTS}struct D\nB[1025] b\nend\nD d\nu8 e\n"
    ));
    assert_eq!(l[0], "root (variable size)");
    assert_eq!(cols(&l[2]), ["0", "?", "D", "d"]);
    assert_eq!(cols(&l[3]), ["?", "1", "u8", "e"]);
    assert!(l.contains(&"struct D (variable size)".to_string()));
    assert!(l.contains(&"struct C (1099511627776 bytes)".to_string()));
}

#[test]
fn chain_of_maximum_counts_overflows_to_unknown() {
    let l = lines(
        "struct A\nbytes[65535] b\nend\nstruct B\nA[65535] a\nend\nstruct C\nB[65535] b\nend\nC c\n",
    );
    assert_eq!(l[0], "root (variable size)");
    assert_eq!(cols(&l[2]), ["0", "?", "C", "c"]);
    assert!(l.contains(&"struct A (65535 bytes)".to_string()));
    assert!(l.contains(&"struct B (4294836225 bytes)".to_string()));
    assert!(l.contains(&"struct C (variable size)".to_string()));
}

#[test]
fn columns_align_to_widest_cell() {
    let l = lines("u8 a\nbytes[1024] a_very_long_field_name\nu8 z\n");
    assert_eq!(l[1], "OFFSET  SIZE  TYPE         FIELD");
    assert_eq!(l[2], "0       1     u8           a");
    assert_eq!(l[3], "1       1024  bytes[1024]  a_very_long_field_name");
    assert_eq!(l[4], "1025    1     u8           z");
}

#[test]
fn schema_error_gives_exit_2_and_no_stdout() {
    let (code, out, err) = layout("foo x\n");
    assert_eq!(code, 2);
    assert_eq!(out, "");
    assert_eq!(err, "bytelens: <stdin>:1: unknown type \"foo\"\n");
}
