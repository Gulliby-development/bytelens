use crate::schema::{Base, Count, Field, IntType, Literal, Schema, StructDef};
use crate::text::{quote, split_lines};

const WS: [char; 2] = [' ', '\t'];
const STAR_MESSAGE: &str = "\"*\" is only allowed on the last field of the file layout";

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

fn next_token(s: &str) -> (&str, &str) {
    let s = s.trim_start_matches(WS);
    match s.find(WS) {
        Some(i) => (&s[..i], s[i..].trim_start_matches(WS)),
        None => (s, ""),
    }
}

/// Parses a schema file. The error is `FILE:LINE: MESSAGE` or `FILE: no fields`.
pub fn parse_schema(display: &str, text: &str) -> Result<Schema, String> {
    let mut schema = Schema::default();
    let mut current: Option<StructDef> = None;
    for (number, raw) in split_lines(text) {
        let line = raw.trim_matches(WS);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let tokens: Vec<&str> = line.split(WS).filter(|t| !t.is_empty()).collect();
        let fail = |message: String| format!("{display}:{number}: {message}");
        match tokens[0] {
            "struct" => {
                if current.is_some() {
                    return Err(fail("struct definitions cannot be nested".to_string()));
                }
                if tokens.len() != 2 {
                    return Err(fail("expected \"struct NAME\"".to_string()));
                }
                if !valid_name(tokens[1]) {
                    return Err(fail(format!("invalid name {}", quote(tokens[1]))));
                }
                if schema.structs.iter().any(|s| s.name == tokens[1]) {
                    return Err(fail(format!("duplicate struct {}", quote(tokens[1]))));
                }
                current = Some(StructDef {
                    name: tokens[1].to_string(),
                    line: number,
                    fields: Vec::new(),
                });
            }
            "end" => {
                if tokens.len() > 1 {
                    return Err(fail("expected \"end\"".to_string()));
                }
                let Some(def) = current.take() else {
                    return Err(fail("\"end\" without struct".to_string()));
                };
                if def.fields.is_empty() {
                    return Err(fail(format!("struct {} has no fields", quote(&def.name))));
                }
                schema.structs.push(def);
            }
            _ => {
                if let Some(star) = schema.root.iter().find(|f| f.count == Some(Count::Rest)) {
                    return Err(format!("{display}:{}: {STAR_MESSAGE}", star.line));
                }
                let scope: &[Field] = match &current {
                    Some(def) => &def.fields,
                    None => &schema.root,
                };
                let field =
                    parse_field(&schema, scope, current.is_some(), number, line).map_err(fail)?;
                match &mut current {
                    Some(def) => def.fields.push(field),
                    None => schema.root.push(field),
                }
            }
        }
    }
    if let Some(def) = current {
        return Err(format!(
            "{display}:{}: struct {} is not closed",
            def.line,
            quote(&def.name)
        ));
    }
    if schema.root.is_empty() {
        return Err(format!("{display}: no fields"));
    }
    Ok(schema)
}

fn parse_field(
    schema: &Schema,
    scope: &[Field],
    in_struct: bool,
    number: usize,
    line: &str,
) -> Result<Field, String> {
    let expected = || "expected \"TYPE NAME [= LITERAL]\"".to_string();
    let (type_text, rest) = next_token(line);
    let (name, rest) = next_token(rest);
    if type_text.is_empty() || name.is_empty() || type_text.contains('=') || name.contains('=') {
        return Err(expected());
    }
    let literal_text = if rest.is_empty() {
        None
    } else {
        let (eq, lit) = next_token(rest);
        if eq != "=" || lit.is_empty() {
            return Err(expected());
        }
        Some(lit)
    };

    let unknown = || format!("unknown type {}", quote(type_text));
    let (base_text, count_text) = match type_text.find('[') {
        Some(i) => {
            let inner = type_text[i + 1..].strip_suffix(']').ok_or_else(unknown)?;
            (&type_text[..i], Some(inner))
        }
        None => (type_text, None),
    };
    let base = if let Some(int) = IntType::from_name(base_text) {
        Base::Int(int)
    } else if base_text == "bytes" {
        Base::Bytes
    } else if base_text == "str" {
        Base::Str
    } else if let Some(i) = schema.structs.iter().position(|s| s.name == base_text) {
        Base::Struct(i)
    } else {
        return Err(unknown());
    };
    let needs_length = matches!(base, Base::Bytes | Base::Str);
    if needs_length && count_text.is_none() {
        return Err("bytes and str need a length".to_string());
    }
    let count = match count_text {
        None => None,
        Some(text) => Some(parse_count(scope, in_struct, text)?),
    };
    if !valid_name(name) {
        return Err(format!("invalid name {}", quote(name)));
    }
    if let Some(first) = scope.iter().find(|f| f.name == name) {
        return Err(format!(
            "duplicate field {} (first on line {})",
            quote(name),
            first.line
        ));
    }
    let literal = match literal_text {
        None => None,
        Some(text) => Some(parse_literal(base, count.as_ref(), text)?),
    };
    Ok(Field {
        name: name.to_string(),
        line: number,
        type_text: type_text.to_string(),
        base,
        count,
        literal,
    })
}

fn parse_count(scope: &[Field], in_struct: bool, text: &str) -> Result<Count, String> {
    let invalid = || format!("invalid count {}", quote(text));
    if text == "*" {
        if in_struct {
            return Err(STAR_MESSAGE.to_string());
        }
        return Ok(Count::Rest);
    }
    if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) {
        if text.len() > 1 && text.starts_with('0') {
            return Err(invalid());
        }
        return match text.parse::<u32>() {
            Ok(n) if n <= 65535 => Ok(Count::Fixed(n)),
            _ => Err(invalid()),
        };
    }
    if !valid_name(text) {
        return Err(invalid());
    }
    match scope.iter().find(|f| f.name == text) {
        None => Err(format!("unknown length field {}", quote(text))),
        Some(f) => match f.base {
            Base::Int(int) if f.count.is_none() && !int.signed() => {
                Ok(Count::Field(text.to_string()))
            }
            _ => Err(format!(
                "length field {} is not an unsigned integer",
                quote(text)
            )),
        },
    }
}

fn parse_int_literal(text: &str) -> Option<Result<i128, String>> {
    if let Some(hex) = text.strip_prefix("0x") {
        if (1..=16).contains(&hex.len()) && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return i128::from_str_radix(hex, 16).ok().map(Ok);
        }
        return None;
    }
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    Some(text.parse::<i128>().map_err(|_| text.to_string()))
}

fn parse_literal(base: Base, count: Option<&Count>, text: &str) -> Result<Literal, String> {
    let invalid = || format!("invalid literal {}", quote(text));
    match (base, count) {
        (Base::Int(int), None) => match parse_int_literal(text) {
            None => Err(invalid()),
            Some(Err(shown)) => Err(format!("value {shown} does not fit {}", int.name())),
            Some(Ok(value)) => {
                let (min, max) = int.range();
                if value < min || value > max {
                    Err(format!("value {value} does not fit {}", int.name()))
                } else {
                    Ok(Literal::Int(value))
                }
            }
        },
        (Base::Bytes | Base::Str, Some(Count::Fixed(n))) => {
            let inner = text
                .strip_prefix('"')
                .and_then(|t| t.strip_suffix('"'))
                .ok_or_else(invalid)?;
            let ok = inner
                .bytes()
                .all(|b| (0x20..=0x7e).contains(&b) && b != b'"' && b != b'\\');
            if !ok {
                return Err(invalid());
            }
            if inner.len() != *n as usize {
                return Err(format!(
                    "string has {} bytes but the field has {n}",
                    inner.len()
                ));
            }
            Ok(Literal::Text(inner.as_bytes().to_vec()))
        }
        _ => Err("a literal needs a number or a fixed-length bytes or str field".to_string()),
    }
}
