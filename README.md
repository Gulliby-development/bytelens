<p align="center"><img src="docs/logo.svg" width="96" alt="" /></p>

# bytelens

[![CI](https://github.com/Gulliby-development/bytelens/actions/workflows/ci.yml/badge.svg)](https://github.com/Gulliby-development/bytelens/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

bytelens is a command-line tool for people who reverse-engineer or debug binary file formats, network captures and test fixtures. A small schema file describes the fields of a file: integers of fixed size and byte order, byte blocks, strings, structs, and arrays whose length comes from an earlier field. Then:

- `bytelens decode SCHEMA FILE` prints every field with its offset and value.
- `bytelens layout SCHEMA` prints the offset and size of every field that is known without data.
- `bytelens dump FILE` prints a hex dump.

It works offline, uses only the Rust standard library and never changes its input.

## Why

When you reverse-engineer a binary format or debug a capture, you want the same answer again and again: where does each field start, how big is it and what does it hold. Write the layout once in a small schema file, then decode any file with it, print the static layout, or fall back to a plain hex dump.

## Install

```
cargo build --release
```

The binary is `target/release/bytelens`.

## Usage

```
usage: bytelens decode SCHEMA FILE
       bytelens layout SCHEMA
       bytelens dump [--width N] FILE
```

Every file argument is a path or `-` for standard input (named `<stdin>`). `--width N` or `--width=N` sets the number of bytes per line for `dump` (1 to 64, default 16).

Worked example. The file `chunks.schema`:

```
# a small container: a header, then records
struct Record
  u8 kind
  u16le size
  bytes[size] data
end

str[4] magic = "CHNK"
u16be version = 2
u8 count
Record[count] records
bytes[*] tail
```

The file `chunks.bin` holds these 18 bytes in hex:

```
43 48 4e 4b 00 02 02 01 03 00 aa bb cc 02 00 00 ff ee
```

`bytelens dump chunks.bin` prints

```
00000000  43 48 4e 4b 00 02 02 01 03 00 aa bb cc 02 00 00  |CHNK............|
00000010  ff ee                                            |..|
```

`bytelens decode chunks.schema chunks.bin` prints

```
00000000  magic = "CHNK"
00000004  version = 2
00000006  count = 2
00000007  records[0]
00000007    kind = 1
00000008    size = 3
0000000a    data = aa bb cc
0000000d  records[1]
0000000d    kind = 2
0000000e    size = 0
00000010    data = (empty)
00000010  tail = ff ee
```

`bytelens layout chunks.schema` prints

```
root (variable size)
OFFSET  SIZE  TYPE           FIELD
0       4     str[4]         magic
4       2     u16be          version
6       1     u8             count
7       ?     Record[count]  records
?       ?     bytes[*]       tail

struct Record (variable size)
OFFSET  SIZE  TYPE         FIELD
0       1     u8           kind
1       2     u16le        size
3       ?     bytes[size]  data
```

## Schema file

The schema is UTF-8 text (one leading BOM is skipped). Lines end with LF or CRLF. Each line is trimmed of spaces and tabs. Empty lines and lines starting with `#` are skipped.

- Names match `[A-Za-z_][A-Za-z0-9_]*` and are not reserved: `struct`, `end`, `bytes`, `str` and the 14 integer type names (`u8` to `i64be`) are invalid as struct and field names.
- `struct NAME` opens a struct and `end` closes it. Structs do not nest and use only structs defined earlier.
- A field is `TYPE NAME` or `TYPE NAME = LITERAL`. Fields outside a struct form the root layout.

Types:

- Integers: `u8`, `i8`, `u16le`, `u16be`, `i16le`, `i16be`, `u32le`, `u32be`, `i32le`, `i32be`, `u64le`, `u64be`, `i64le`, `i64be` (`le` is little endian, `be` is big endian).
- `bytes` and `str`, which need a count (`bytes[N]`, `str[N]`, N bytes).
- A struct name.

Counts: `TYPE[COUNT]` makes an array. COUNT is a number from 0 to 65535 without a leading zero, the name of an earlier unsigned integer field (a scalar) of the same struct or root, or `*` (to the end of the data; only on the last field of the root).

Literals: a scalar integer field may have a decimal literal (`123` or `-5`, no leading zero) or a hex literal (`0x1F`, 1 to 16 hex digits). A `bytes[N]` or `str[N]` field with a number count may have a quoted string literal of exactly N bytes from 0x20 to 0x7E, except `"` and `\`. Decoding fails when the data differs from the literal.

## Output

- `decode` prints one line per field: the offset as 8 lowercase hex digits, two spaces, two more spaces per nesting level, then `NAME = VALUE`. Integers are decimal. `bytes` show lowercase hex pairs (the first 16, then ` ... (N bytes)`, or `(empty)`). `str` shows the bytes without trailing NULs in double quotes, with `"` and `\` escaped and `\xHH` for control characters. Integer arrays are shown as `[1, 2, 3]` (the first 16 items, then `, ... (N items)`). A struct prints its name and its fields one level deeper; a struct array prints `NAME[0]`, `NAME[1]` and so on.
- When the data does not fit the schema, the lines printed so far stay on standard output and `bytelens: FILE: offset 0xHHHHHHHH: TEXT` goes to standard error.
- `layout` prints a section for the root and one for every struct in definition order, each with a heading and a table of OFFSET, SIZE, TYPE and FIELD. Unknown sizes and offsets are shown as `?`.
- `dump` prints lines of `--width` bytes: the offset, the bytes as hex pairs and the printable bytes between `|` characters.

Every line ends with LF on every operating system.

## Exit codes

- 0: success
- 1: the data does not fit the schema
- 2: usage or input error

Usage and input errors print `bytelens: MESSAGE` on standard error and nothing on standard output.

## Development

```
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

The project uses only the standard library and has no dependencies.

## License

MIT, see [LICENSE](LICENSE).

---

Built with BAKED, an open-source coding agent (bench loop round 26, BAKED 628b12fc).
