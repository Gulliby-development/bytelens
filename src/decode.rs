use std::collections::HashMap;

use crate::schema::{Base, Count, Field, IntType, Literal, Schema, StructDef};
use crate::text::{hex_pairs, hex8};
use crate::value::{format_int, format_int_array, read_int};

/// A decoding failure: the offset and the message text.
type Failure = (usize, String);

/// Decodes `data` with `schema`. Returns the lines printed so far and, on failure,
/// the error message without the `bytelens: ` prefix.
pub fn decode(schema: &Schema, data: &[u8], display: &str) -> (Vec<String>, Option<String>) {
    let mut decoder = Decoder {
        schema,
        data,
        lines: Vec::new(),
    };
    let mut pos = 0;
    let mut result = decoder.fields(&schema.root, &mut pos, 0, "");
    if result.is_ok() && pos < data.len() {
        let left = data.len() - pos;
        let text = if left == 1 {
            "1 trailing byte after the last field".to_string()
        } else {
            format!("{left} trailing bytes after the last field")
        };
        result = Err((pos, text));
    }
    let error = result
        .err()
        .map(|(offset, text)| format!("{display}: offset 0x{}: {text}", hex8(offset as u64)));
    (decoder.lines, error)
}

struct Decoder<'a> {
    schema: &'a Schema,
    data: &'a [u8],
    lines: Vec<String>,
}

fn resolve(
    count: &Count,
    scope: &HashMap<&str, i128>,
    pos: usize,
    path: &str,
) -> Result<Option<usize>, Failure> {
    match count {
        Count::Fixed(n) => Ok(Some(*n as usize)),
        Count::Field(name) => {
            let value = scope.get(name.as_str()).copied().unwrap_or(0);
            if value > 65535 {
                Err((pos, format!("count {value} of \"{path}\" exceeds 65535")))
            } else {
                Ok(Some(value as usize))
            }
        }
        Count::Rest => Ok(None),
    }
}

fn int_literal(field: &Field) -> Option<i128> {
    match &field.literal {
        Some(Literal::Int(value)) => Some(*value),
        _ => None,
    }
}

fn text_literal(field: &Field) -> Option<&[u8]> {
    match &field.literal {
        Some(Literal::Text(bytes)) => Some(bytes.as_slice()),
        _ => None,
    }
}

fn str_display(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn bytes_display(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "(empty)".to_string();
    }
    let mut out = hex_pairs(&bytes[..bytes.len().min(16)]);
    if bytes.len() > 16 {
        out.push_str(&format!(" ... ({} bytes)", bytes.len()));
    }
    out
}

impl<'a> Decoder<'a> {
    fn emit(&mut self, offset: usize, level: usize, text: &str) {
        self.lines.push(format!(
            "{}  {}{}",
            hex8(offset as u64),
            "  ".repeat(level),
            text
        ));
    }

    fn take(&self, pos: usize, n: usize, path: &str) -> Result<&'a [u8], Failure> {
        let remaining = self.data.len() - pos;
        if n > remaining {
            return Err((
                pos,
                format!("need {n} bytes for \"{path}\", only {remaining} left"),
            ));
        }
        Ok(&self.data[pos..pos + n])
    }

    fn fields(
        &mut self,
        fields: &[Field],
        pos: &mut usize,
        level: usize,
        prefix: &str,
    ) -> Result<(), Failure> {
        let mut scope: HashMap<&str, i128> = HashMap::new();
        for field in fields {
            let path = if prefix.is_empty() {
                field.name.clone()
            } else {
                format!("{prefix}.{}", field.name)
            };
            match (field.base, &field.count) {
                (Base::Int(ty), None) => {
                    let value = self.scalar(field, ty, pos, level, &path)?;
                    scope.insert(field.name.as_str(), value);
                }
                (Base::Int(ty), Some(count)) => {
                    let n = resolve(count, &scope, *pos, &path)?;
                    self.int_array(field, ty, n, pos, level, &path)?;
                }
                (Base::Bytes | Base::Str, count) => {
                    let n = match count {
                        Some(count) => resolve(count, &scope, *pos, &path)?,
                        None => None,
                    };
                    self.block(field, n, pos, level, &path)?;
                }
                (Base::Struct(index), None) => {
                    let schema = self.schema;
                    self.emit(*pos, level, &field.name);
                    self.fields(&schema.structs[index].fields, pos, level + 1, &path)?;
                }
                (Base::Struct(index), Some(count)) => {
                    let n = resolve(count, &scope, *pos, &path)?;
                    self.struct_array(field, index, n, pos, level, &path)?;
                }
            }
        }
        Ok(())
    }

    fn scalar(
        &mut self,
        field: &Field,
        ty: IntType,
        pos: &mut usize,
        level: usize,
        path: &str,
    ) -> Result<i128, Failure> {
        let start = *pos;
        let bytes = self.take(start, ty.size(), path)?;
        let value = read_int(ty, bytes).unwrap_or_default();
        if let Some(expected) = int_literal(field).filter(|l| *l != value) {
            return Err((
                start,
                format!(
                    "\"{path}\" is {}, expected {}",
                    format_int(value),
                    format_int(expected)
                ),
            ));
        }
        self.emit(
            start,
            level,
            &format!("{} = {}", field.name, format_int(value)),
        );
        *pos = start + ty.size();
        Ok(value)
    }

    fn int_array(
        &mut self,
        field: &Field,
        ty: IntType,
        n: Option<usize>,
        pos: &mut usize,
        level: usize,
        path: &str,
    ) -> Result<(), Failure> {
        let size = ty.size();
        let start = *pos;
        let values: Vec<i128> = match n {
            Some(n) => {
                let bytes = self.take(start, n * size, path)?;
                bytes
                    .chunks(size)
                    .map(|c| read_int(ty, c).unwrap_or_default())
                    .collect()
            }
            None => {
                let mut values = Vec::new();
                let mut p = start;
                while p < self.data.len() {
                    let item_path = format!("{path}[{}]", values.len());
                    let bytes = self.take(p, size, &item_path)?;
                    values.push(read_int(ty, bytes).unwrap_or_default());
                    p += size;
                }
                values
            }
        };
        *pos = start + values.len() * size;
        self.emit(
            start,
            level,
            &format!("{} = {}", field.name, format_int_array(&values)),
        );
        Ok(())
    }

    fn block(
        &mut self,
        field: &Field,
        n: Option<usize>,
        pos: &mut usize,
        level: usize,
        path: &str,
    ) -> Result<(), Failure> {
        let start = *pos;
        let n = n.unwrap_or(self.data.len() - start);
        let bytes = self.take(start, n, path)?;
        let expected = text_literal(field).filter(|l| *l != bytes);
        let shown = if field.base == Base::Str {
            let end = bytes.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
            let text = std::str::from_utf8(&bytes[..end])
                .map_err(|_| (start, format!("invalid UTF-8 in \"{path}\"")))?;
            if let Some(literal) = expected {
                return Err((
                    start,
                    format!(
                        "\"{path}\" is {}, expected {}",
                        str_display(text),
                        str_display(&String::from_utf8_lossy(literal))
                    ),
                ));
            }
            str_display(text)
        } else {
            if let Some(literal) = expected {
                return Err((
                    start,
                    format!(
                        "\"{path}\" is {}, expected {}",
                        hex_pairs(bytes),
                        hex_pairs(literal)
                    ),
                ));
            }
            bytes_display(bytes)
        };
        self.emit(start, level, &format!("{} = {shown}", field.name));
        *pos = start + n;
        Ok(())
    }

    fn element(
        &mut self,
        field: &Field,
        def: &StructDef,
        k: usize,
        pos: &mut usize,
        level: usize,
        path: &str,
    ) -> Result<(), Failure> {
        self.emit(*pos, level, &format!("{}[{k}]", field.name));
        self.fields(&def.fields, pos, level + 1, &format!("{path}[{k}]"))
    }

    fn struct_array(
        &mut self,
        field: &Field,
        index: usize,
        n: Option<usize>,
        pos: &mut usize,
        level: usize,
        path: &str,
    ) -> Result<(), Failure> {
        let schema = self.schema;
        let def = &schema.structs[index];
        let mut count = 0;
        match n {
            Some(n) => {
                while count < n {
                    self.element(field, def, count, pos, level, path)?;
                    count += 1;
                }
            }
            None => {
                while *pos < self.data.len() {
                    let start = *pos;
                    self.element(field, def, count, pos, level, path)?;
                    if *pos == start {
                        return Err((start, format!("\"{path}[{count}]\" is empty")));
                    }
                    count += 1;
                }
            }
        }
        if count == 0 {
            self.emit(*pos, level, &format!("{} = []", field.name));
        }
        Ok(())
    }
}
