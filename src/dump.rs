use crate::text::{hex_pairs, hex8};

/// Hex dump lines of `width` bytes each; empty data gives no lines.
pub fn dump(data: &[u8], width: usize) -> Vec<String> {
    data.chunks(width.max(1))
        .enumerate()
        .map(|(i, chunk)| {
            let ascii: String = chunk
                .iter()
                .map(|b| {
                    if (0x20..=0x7e).contains(b) {
                        *b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            format!(
                "{}  {:<pad$}  |{}|",
                hex8((i * width.max(1)) as u64),
                hex_pairs(chunk),
                ascii,
                pad = 3 * width.max(1) - 1
            )
        })
        .collect()
}
