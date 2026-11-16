use super::{Detector, Finding};

/// Reason code reported for every invisible or bidirectional character.
pub(crate) const REASON_CODE: &str = "content.invisible_unicode";

const DETECTOR_NAME: &str = "invisible_unicode";

/// Flags zero-width and bidirectional control characters ("Trojan Source").
///
/// These characters are invisible when rendered but change how text is read
/// or interpreted, so prompts containing them are treated as hostile.
pub(crate) struct InvisibleUnicode;

impl Detector for InvisibleUnicode {
    fn scan(&self, content: &str) -> Vec<Finding> {
        content
            .char_indices()
            .filter(|(_, character)| is_invisible(*character))
            .map(|(start, character)| Finding {
                code: REASON_CODE,
                detector: DETECTOR_NAME,
                start,
                end: start + character.len_utf8(),
            })
            .collect()
    }
}

/// Zero-width spaces/joiners, line/paragraph separators, bidirectional
/// embedding and isolate controls, and the byte-order mark.
fn is_invisible(character: char) -> bool {
    matches!(
        character,
        '\u{200B}'..='\u{200F}'
            | '\u{2028}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
}

#[cfg(test)]
mod tests {
    use super::{Detector, Finding, InvisibleUnicode, DETECTOR_NAME, REASON_CODE};

    fn finding(start: usize, end: usize) -> Finding {
        Finding {
            code: REASON_CODE,
            detector: DETECTOR_NAME,
            start,
            end,
        }
    }

    #[test]
    fn clean_text_has_no_findings() {
        assert!(InvisibleUnicode
            .scan("Olá, tudo bem? Hello there!")
            .is_empty());
        assert!(InvisibleUnicode.scan("").is_empty());
    }

    #[test]
    fn reports_byte_offsets_after_multibyte_characters() {
        // "olá" is 4 bytes (á = 2 bytes); U+200B is 3 bytes in UTF-8.
        assert_eq!(
            InvisibleUnicode.scan("olá\u{200B}mundo"),
            vec![finding(4, 7)]
        );
    }

    #[test]
    fn reports_every_occurrence_in_order() {
        assert_eq!(
            InvisibleUnicode.scan("\u{FEFF}ab\u{202E}c\u{2066}"),
            vec![finding(0, 3), finding(5, 8), finding(9, 12)]
        );
    }

    #[test]
    fn covers_range_boundaries_only() {
        for flagged in [
            '\u{200B}', '\u{200F}', '\u{2028}', '\u{202E}', '\u{2060}', '\u{2064}', '\u{2066}',
            '\u{2069}', '\u{FEFF}',
        ] {
            assert_eq!(
                InvisibleUnicode.scan(&flagged.to_string()).len(),
                1,
                "U+{:04X}",
                flagged as u32
            );
        }
        for allowed in [
            '\u{200A}', '\u{2010}', '\u{2027}', '\u{202F}', '\u{205F}', '\u{2065}', '\u{206A}',
            '\u{FEFE}', 'a', ' ', '\n',
        ] {
            assert!(
                InvisibleUnicode.scan(&allowed.to_string()).is_empty(),
                "U+{:04X}",
                allowed as u32
            );
        }
    }
}
