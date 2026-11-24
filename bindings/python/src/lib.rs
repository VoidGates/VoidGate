//! Python module `voidgate._voidgate`: the scan only. Policy (what blocks,
//! placeholders, conversations, SDK wrappers) lives in `voidgate/__init__.py`.
//!
//! Offsets are converted from UTF-8 bytes (Rust) to code points (Python), so
//! `text[start:end]` is the span in Python too.

use pyo3::prelude::*;

/// Code-point index of each byte offset, in one pass over the text whatever
/// the number of offsets. Counts lead bytes instead of slicing, so an offset
/// inside a character or past the end can never panic.
fn char_indexes(text: &str, offsets: &[usize]) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut order: Vec<usize> = (0..offsets.len()).collect();
    order.sort_unstable_by_key(|&i| offsets[i]);
    let mut out = vec![0; offsets.len()];
    let (mut position, mut count) = (0, 0);
    for i in order {
        let target = offsets[i].min(bytes.len());
        while position < target {
            if bytes[position] & 0xC0 != 0x80 {
                count += 1;
            }
            position += 1;
        }
        out[i] = count;
    }
    out
}

/// Every finding in `text` as `(code, detector, kind, start, end)`, ordered by
/// offset. The GIL is released while scanning, so threads run in parallel.
#[pyfunction]
fn scan(
    py: Python<'_>,
    text: String,
) -> Vec<(&'static str, &'static str, &'static str, usize, usize)> {
    py.detach(|| {
        let findings = voidgate_core::inspect(&text).findings;
        let offsets: Vec<usize> = findings.iter().flat_map(|f| [f.start, f.end]).collect();
        let indexes = char_indexes(&text, &offsets);
        findings
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    f.code,
                    f.detector,
                    f.kind,
                    indexes[2 * i],
                    indexes[2 * i + 1],
                )
            })
            .collect()
    })
}

#[pymodule]
fn _voidgate(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(scan, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::char_indexes;

    fn char_index(text: &str, byte: usize) -> usize {
        char_indexes(text, &[byte])[0]
    }

    #[test]
    fn byte_offsets_become_code_point_indexes() {
        let text = "ação 🔒 x";
        assert_eq!(char_index(text, 0), 0);
        assert_eq!(char_index(text, "ação".len()), 4);
        assert_eq!(char_index(text, "ação 🔒".len()), 6);
        assert_eq!(char_index(text, text.len()), 8);
    }

    #[test]
    fn offset_inside_a_character_or_past_the_end_does_not_panic() {
        let text = "ç";
        assert_eq!(char_index(text, 1), 1);
        assert_eq!(char_index(text, 99), 1);
    }

    #[test]
    fn offsets_in_any_order_keep_their_place() {
        let text = "ação 🔒 x";
        let offsets = [text.len(), 0, "ação".len(), "ação".len(), 99];
        assert_eq!(char_indexes(text, &offsets), vec![8, 0, 4, 4, 8]);
        assert!(char_indexes(text, &[]).is_empty());
    }
}
