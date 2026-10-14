use crate::schema::{Base, Count, Field, Schema};

const CAP: u64 = 1 << 40;

fn field_size(field: &Field, struct_sizes: &[Option<u64>]) -> Option<u64> {
    let element = match field.base {
        Base::Int(ty) => ty.size() as u64,
        Base::Bytes | Base::Str => 1,
        Base::Struct(index) => struct_sizes[index]?,
    };
    let size = match &field.count {
        None => element,
        Some(Count::Fixed(n)) => element.checked_mul(u64::from(*n))?,
        Some(Count::Field(_) | Count::Rest) => return None,
    };
    (size <= CAP).then_some(size)
}

fn cell(value: Option<u64>) -> String {
    value.map_or_else(|| "?".to_string(), |v| v.to_string())
}

/// Returns the table rows and the total size of a field list.
fn section(fields: &[Field], struct_sizes: &[Option<u64>]) -> (Vec<[String; 4]>, Option<u64>) {
    let mut rows = vec![[
        "OFFSET".to_string(),
        "SIZE".to_string(),
        "TYPE".to_string(),
        "FIELD".to_string(),
    ]];
    let mut offset = Some(0u64);
    for field in fields {
        let size = field_size(field, struct_sizes);
        rows.push([
            cell(offset),
            cell(size),
            field.type_text.clone(),
            field.name.clone(),
        ]);
        offset = match (offset, size) {
            (Some(o), Some(s)) if o + s <= CAP => Some(o + s),
            _ => None,
        };
    }
    (rows, offset)
}

fn render(heading: &str, total: Option<u64>, rows: &[[String; 4]], out: &mut Vec<String>) {
    let size = match total {
        Some(1) => "1 byte".to_string(),
        Some(n) => format!("{n} bytes"),
        None => "variable size".to_string(),
    };
    out.push(format!("{heading} ({size})"));
    let mut widths = [0usize; 4];
    for row in rows {
        for (width, text) in widths.iter_mut().zip(row) {
            *width = (*width).max(text.chars().count());
        }
    }
    for row in rows {
        let mut line = String::new();
        for (i, text) in row.iter().enumerate() {
            if i > 0 {
                line.push_str("  ");
            }
            line.push_str(text);
            line.push_str(&" ".repeat(widths[i] - text.chars().count()));
        }
        out.push(line.trim_end().to_string());
    }
}

/// The layout tables of the root and of every struct in definition order.
pub fn layout(schema: &Schema) -> Vec<String> {
    let mut struct_sizes: Vec<Option<u64>> = Vec::new();
    let mut tables = Vec::new();
    for def in &schema.structs {
        let (rows, total) = section(&def.fields, &struct_sizes);
        struct_sizes.push(total);
        tables.push((format!("struct {}", def.name), total, rows));
    }
    let (rows, total) = section(&schema.root, &struct_sizes);
    let mut out = Vec::new();
    render("root", total, &rows, &mut out);
    for (heading, total, rows) in &tables {
        out.push(String::new());
        render(heading, *total, rows, &mut out);
    }
    out
}
