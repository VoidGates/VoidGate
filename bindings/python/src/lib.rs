//! Python module `voidgate._voidgate`, re-exported by `voidgate/__init__.py`.
//!
//! Offsets are converted from UTF-8 bytes (Rust) to code points (Python), so
//! `text[f.start:f.end]` is the span in Python too.

use pyo3::prelude::*;

/// One suspicious span. `text[start:end]` is the span; the finding never
/// carries the text itself.
#[pyclass(frozen, get_all, module = "voidgate")]
#[derive(Clone)]
struct Finding {
    code: &'static str,
    detector: &'static str,
    start: usize,
    end: usize,
}

#[pymethods]
impl Finding {
    fn __repr__(&self) -> String {
        format!(
            "Finding(code='{}', detector='{}', start={}, end={})",
            self.code, self.detector, self.start, self.end
        )
    }
}

/// What a text contains and what to do with it.
#[pyclass(frozen, get_all, module = "voidgate")]
struct Inspection {
    /// "allow", "mask" or "block".
    action: &'static str,
    findings: Vec<Finding>,
    /// The text with personal data replaced by "[redacted]".
    redacted: String,
}

#[pymethods]
impl Inspection {
    fn __repr__(&self) -> String {
        format!(
            "Inspection(action='{}', findings={})",
            self.action,
            self.findings.len()
        )
    }
}

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

/// Inspects `text` with every built-in detector.
#[pyfunction]
fn inspect(text: &str) -> Inspection {
    let found = voidgate_core::inspect(text);
    let redacted = voidgate_core::redact(text, &found.findings);
    let offsets: Vec<usize> = found
        .findings
        .iter()
        .flat_map(|f| [f.start, f.end])
        .collect();
    let indexes = char_indexes(text, &offsets);
    let findings = found
        .findings
        .iter()
        .enumerate()
        .map(|(i, f)| Finding {
            code: f.code,
            detector: f.detector,
            start: indexes[2 * i],
            end: indexes[2 * i + 1],
        })
        .collect();
    Inspection {
        action: found.action.as_str(),
        findings,
        redacted,
    }
}

/// `text` with personal data replaced by "[redacted]".
#[pyfunction]
fn redact(text: &str) -> String {
    voidgate_core::redact(text, &voidgate_core::inspect(text).findings)
}

#[pymodule]
fn _voidgate(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Finding>()?;
    m.add_class::<Inspection>()?;
    m.add_function(wrap_pyfunction!(inspect, m)?)?;
    m.add_function(wrap_pyfunction!(redact, m)?)?;
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
