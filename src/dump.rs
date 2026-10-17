use crate::text::{hex_pairs, hex8};

/// Hex dump lines of `width` bytes each; empty data gives no lines.
pub fn dump(data: &[u8], width: usize) -> Vec<String> {
    dump_from(data, width, 0)
}

/// Hex dump lines starting at `skip`, retaining offsets from the original data.
pub fn dump_from(data: &[u8], width: usize, skip: usize) -> Vec<String> {
    if skip >= data.len() {
        return Vec::new();
    }

    let chunk_width = width.max(1);
    data[skip..]
        .chunks(chunk_width)
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
                hex8((skip + i * chunk_width) as u64),
                hex_pairs(chunk),
                ascii,
                pad = 3 * chunk_width - 1
            )
        })
        .collect()
}
