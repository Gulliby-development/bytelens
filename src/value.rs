use crate::schema::IntType;

/// Reads an integer of type `ty` from the start of `bytes` in its byte order.
/// Returns `None` when fewer than `ty.size()` bytes are given.
pub fn read_int(ty: IntType, bytes: &[u8]) -> Option<i128> {
    let size = ty.size();
    let slice = bytes.get(..size)?;
    let mut raw: u128 = 0;
    if ty.big_endian() {
        for b in slice {
            raw = (raw << 8) | u128::from(*b);
        }
    } else {
        for b in slice.iter().rev() {
            raw = (raw << 8) | u128::from(*b);
        }
    }
    let bits = size as u32 * 8;
    if ty.signed() && ((raw >> (bits - 1)) & 1) == 1 {
        Some(raw as i128 - (1i128 << bits))
    } else {
        Some(raw as i128)
    }
}

/// Decimal display of an integer.
pub fn format_int(value: i128) -> String {
    value.to_string()
}

/// Display of an integer array: the first 16 items, then `, ... (N items)`; `[]` when empty.
pub fn format_int_array(values: &[i128]) -> String {
    let shown: Vec<String> = values.iter().take(16).map(|v| format_int(*v)).collect();
    let mut out = String::from("[");
    out.push_str(&shown.join(", "));
    if values.len() > 16 {
        out.push_str(&format!(", ... ({} items)", values.len()));
    }
    out.push(']');
    out
}
