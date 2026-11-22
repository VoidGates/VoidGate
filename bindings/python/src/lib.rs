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

/// Code-point index of a byte offset. Counts lead bytes instead of slicing,
/// so an offset inside a character can never panic.
fn char_index(text: &str, byte: usize) -> usize {
    let bytes = &text.as_bytes()[..byte.min(text.len())];
    bytes.iter().filter(|b| (**b & 0xC0) != 0x80).count()
}

/// Inspects `text` with every built-in detector.
#[pyfunction]
fn inspect(text: &str) -> Inspection {
    let found = voidgate_core::inspect(text);
    let redacted = voidgate_core::redact(text, &found.findings);
    // shortcut: counts chars from the start for each offset (O(n·k)); fine for
    // chat-sized text, upgrade to one forward pass if findings get large.
    let findings = found
        .findings
        .iter()
        .map(|f| Finding {
            code: f.code,
            detector: f.detector,
            start: char_index(text, f.start),
            end: char_index(text, f.end),
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
    use super::char_index;

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
}
