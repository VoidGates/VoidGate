//! Personal-data spans. The decision is `mask`: offsets only, never the value.
//!
//! shortcut: email, E.164 phone, Luhn card, IBAN for the countries listed in
//! `iban_len`, Brazilian CPF/CNPJ and US SSN. Add a country when a pilot needs it.

use super::{Detector, Finding};

pub const REASON_CODE: &str = "content.personal_data";

const DETECTOR_NAME: &str = "personal_data";
const REDACTED: &str = "[redacted]";

pub struct PersonalData;

impl Detector for PersonalData {
    fn scan(&self, content: &str) -> Vec<Finding> {
        let bytes = content.as_bytes();
        let mut findings = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            let end = try_phone(bytes, index)
                .or_else(|| try_iban(bytes, index))
                .or_else(|| try_email(bytes, index))
                .or_else(|| try_ssn(bytes, index))
                .or_else(|| try_br_phone(bytes, index))
                .or_else(|| try_id_or_card(bytes, index));
            if let Some(end) = end {
                findings.push(Finding {
                    code: REASON_CODE,
                    detector: DETECTOR_NAME,
                    start: index,
                    end,
                });
                index = end;
            } else {
                index += 1;
            }
        }
        findings
    }
}

/// Replaces personal-data spans with `[redacted]`. Other findings are ignored.
pub fn redact(content: &str, findings: &[Finding]) -> String {
    let mut spans: Vec<(usize, usize)> = findings
        .iter()
        .filter(|finding| finding.code == REASON_CODE)
        .map(|finding| (finding.start, finding.end))
        .filter(|(start, end)| {
            *start <= *end
                && *end <= content.len()
                && content.is_char_boundary(*start)
                && content.is_char_boundary(*end)
        })
        .collect();
    spans.sort_unstable();
    let mut out = String::with_capacity(content.len());
    let mut cursor = 0;
    for (start, end) in spans {
        if start < cursor {
            continue;
        }
        out.push_str(&content[cursor..start]);
        out.push_str(REDACTED);
        cursor = end;
    }
    out.push_str(&content[cursor..]);
    out
}

fn try_phone(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes[start] != b'+' {
        return None;
    }
    if start > 0 && bytes[start - 1].is_ascii_digit() {
        return None;
    }
    let mut index = start + 1;
    let mut digits = 0u8;
    let mut first = None;
    while index < bytes.len() && digits < 16 {
        let byte = bytes[index];
        if byte.is_ascii_digit() {
            if first.is_none() {
                first = Some(byte);
            }
            digits += 1;
            index += 1;
            continue;
        }
        if matches!(byte, b' ' | b'-' | b'.' | b'(' | b')')
            && digits > 0
            && index + 1 < bytes.len()
            && bytes[index + 1].is_ascii_digit()
        {
            index += 1;
            continue;
        }
        break;
    }
    if (8..=15).contains(&digits) && first.is_some_and(|digit| digit != b'0') {
        Some(index)
    } else {
        None
    }
}

fn try_email(bytes: &[u8], start: usize) -> Option<usize> {
    if start > 0 && is_local(bytes[start - 1]) {
        return None;
    }
    if !bytes[start].is_ascii_alphanumeric() {
        return None;
    }
    let mut index = start;
    while index < bytes.len() && is_local(bytes[index]) {
        index += 1;
    }
    if index == start || bytes[index - 1] == b'.' || index >= bytes.len() || bytes[index] != b'@' {
        return None;
    }
    index += 1;
    let mut dots = 0;
    let mut label = 0;
    let domain_start = index;
    while index < bytes.len() && (is_domain(bytes[index]) || bytes[index] == b'.') {
        if bytes[index] == b'.' {
            if label == 0 {
                return None;
            }
            dots += 1;
            label = 0;
        } else {
            label += 1;
        }
        index += 1;
    }
    if dots < 1 || label < 2 || bytes[domain_start] == b'-' || bytes[index - 1] == b'.' {
        return None;
    }
    Some(index)
}

fn try_ssn(bytes: &[u8], start: usize) -> Option<usize> {
    if start + 11 > bytes.len() {
        return None;
    }
    if start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'-') {
        return None;
    }
    let slice = &bytes[start..start + 11];
    if slice[3] != b'-' || slice[6] != b'-' {
        return None;
    }
    if !slice
        .iter()
        .enumerate()
        .all(|(pos, byte)| matches!(pos, 3 | 6) || byte.is_ascii_digit())
    {
        return None;
    }
    let end = start + 11;
    if end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'-') {
        return None;
    }
    let area = number(&slice[0..3]);
    let group = number(&slice[4..6]);
    let serial = number(&slice[7..11]);
    if area == 0 || area == 666 || area >= 900 || group == 0 || serial == 0 {
        return None;
    }
    Some(end)
}

fn try_iban(bytes: &[u8], start: usize) -> Option<usize> {
    if !bytes[start].is_ascii_alphabetic() {
        return None;
    }
    if start > 0 && bytes[start - 1].is_ascii_alphanumeric() {
        return None;
    }
    if start + 1 >= bytes.len() || !bytes[start + 1].is_ascii_alphabetic() {
        return None;
    }
    let mut index = start;
    let mut compact = String::new();
    while index < bytes.len() && compact.len() < 34 {
        let byte = bytes[index];
        if byte.is_ascii_alphanumeric() {
            compact.push(byte.to_ascii_uppercase() as char);
            index += 1;
        } else if byte == b' '
            && !compact.is_empty()
            && index + 1 < bytes.len()
            && bytes[index + 1].is_ascii_alphanumeric()
        {
            index += 1;
        } else {
            break;
        }
    }
    if compact.len() < 2 {
        return None;
    }
    let expected = iban_len(&compact[..2])?;
    if compact.len() != expected || !iban_checksum(&compact) {
        return None;
    }
    Some(index)
}

/// Brazilian phone as people type it: `(11) 98765-4321`, `(11)3456-7890`,
/// `11 98765-4321`, `98765-4321`. Needs the parentheses or the hyphen, so a
/// bare run of digits (order id, CPF) is left to the other checks.
fn try_br_phone(bytes: &[u8], start: usize) -> Option<usize> {
    if start > 0 && bytes[start - 1].is_ascii_digit() {
        return None;
    }
    let ddd = |at: usize| {
        let pair = bytes.get(at..at + 2)?;
        (pair.iter().all(|b| (b'1'..=b'9').contains(b))).then_some(at + 2)
    };
    let mut index = start;
    let mut parenthesized = false;
    if bytes[index] == b'(' {
        index = ddd(index + 1)?;
        if bytes.get(index) != Some(&b')') {
            return None;
        }
        index += 1;
        if bytes.get(index) == Some(&b' ') {
            index += 1;
        }
        parenthesized = true;
    } else if let Some(after) = ddd(index).filter(|&at| bytes.get(at) == Some(&b' ')) {
        index = after + 1;
    }
    // Mobile: 9 + 4 digits; landline: 4 digits starting with 2-5.
    let head = match bytes.get(index) {
        Some(b'9') => 5,
        Some(b'2'..=b'5') => 4,
        _ => return None,
    };
    let head_end = index + head;
    if !bytes.get(index..head_end)?.iter().all(u8::is_ascii_digit) {
        return None;
    }
    index = head_end;
    let hyphen = bytes.get(index) == Some(&b'-');
    if hyphen || (parenthesized && bytes.get(index) == Some(&b' ')) {
        index += 1;
    }
    if !hyphen && !parenthesized {
        return None;
    }
    let end = index + 4;
    if !bytes.get(index..end)?.iter().all(u8::is_ascii_digit) {
        return None;
    }
    if bytes.get(end).is_some_and(u8::is_ascii_digit) {
        return None;
    }
    Some(end)
}

fn try_id_or_card(bytes: &[u8], start: usize) -> Option<usize> {
    if !bytes[start].is_ascii_digit() {
        return None;
    }
    if start > 0 && bytes[start - 1].is_ascii_digit() {
        return None;
    }
    if let Some(end) = take(bytes, start, 11) {
        if cpf_ok(&only_digits(&bytes[start..end])) {
            return Some(end);
        }
    }
    if let Some(end) = take(bytes, start, 14) {
        if cnpj_ok(&only_digits(&bytes[start..end])) {
            return Some(end);
        }
    }
    for length in (13..=19).rev() {
        let Some(end) = take(bytes, start, length) else {
            continue;
        };
        let digits = only_digits(&bytes[start..end]);
        if digits.iter().all(|digit| digit == &digits[0]) {
            continue;
        }
        let bare = bytes[start..end].iter().all(u8::is_ascii_digit);
        // A bare 13–15 or 17–19 digit run is usually an order id. 16 is the common card length.
        if bare && length != 16 {
            continue;
        }
        if luhn(&digits) {
            return Some(end);
        }
    }
    None
}

fn take(bytes: &[u8], start: usize, count: usize) -> Option<usize> {
    let mut index = start;
    let mut got = 0;
    while got < count {
        if index >= bytes.len() {
            return None;
        }
        if bytes[index].is_ascii_digit() {
            got += 1;
            index += 1;
            continue;
        }
        if matches!(bytes[index], b' ' | b'.' | b'-' | b'/')
            && got > 0
            && index + 1 < bytes.len()
            && bytes[index + 1].is_ascii_digit()
        {
            index += 1;
            continue;
        }
        return None;
    }
    if index < bytes.len() && bytes[index].is_ascii_digit() {
        return None;
    }
    Some(index)
}

fn only_digits(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().copied().filter(u8::is_ascii_digit).collect()
}

fn number(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(0, |acc, byte| acc * 10 + u32::from(byte - b'0'))
}

fn luhn(digits: &[u8]) -> bool {
    let mut sum = 0;
    let mut double = false;
    for digit in digits.iter().rev() {
        let mut value = u32::from(digit - b'0');
        if double {
            value *= 2;
            if value > 9 {
                value -= 9;
            }
        }
        sum += value;
        double = !double;
    }
    sum % 10 == 0
}

fn cpf_ok(digits: &[u8]) -> bool {
    if digits.len() != 11 || digits.iter().all(|digit| digit == &digits[0]) {
        return false;
    }
    let nums: Vec<u32> = digits.iter().map(|digit| u32::from(digit - b'0')).collect();
    let check = |length: usize| {
        let sum: u32 = (0..length)
            .map(|index| nums[index] * (length as u32 + 1 - index as u32))
            .sum();
        (sum * 10) % 11 % 10
    };
    check(9) == nums[9] && check(10) == nums[10]
}

fn cnpj_ok(digits: &[u8]) -> bool {
    if digits.len() != 14 || digits.iter().all(|digit| digit == &digits[0]) {
        return false;
    }
    let nums: Vec<u32> = digits.iter().map(|digit| u32::from(digit - b'0')).collect();
    let weigh = |weights: &[u32]| {
        let sum: u32 = weights
            .iter()
            .enumerate()
            .map(|(index, weight)| nums[index] * weight)
            .sum();
        let rest = sum % 11;
        if rest < 2 {
            0
        } else {
            11 - rest
        }
    };
    weigh(&[5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]) == nums[12]
        && weigh(&[6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]) == nums[13]
}

fn is_local(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'%' | b'+' | b'-')
}

fn is_domain(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

fn iban_len(country: &str) -> Option<usize> {
    Some(match country {
        "AL" => 28,
        "AD" => 24,
        "AT" => 20,
        "AZ" => 28,
        "BH" => 22,
        "BE" => 16,
        "BA" => 20,
        "BR" => 29,
        "BG" => 22,
        "CR" => 22,
        "HR" => 21,
        "CY" => 28,
        "CZ" => 24,
        "DK" => 18,
        "DO" => 28,
        "EE" => 20,
        "FI" => 18,
        "FR" => 27,
        "GE" => 22,
        "DE" => 22,
        "GI" => 23,
        "GR" => 27,
        "GT" => 28,
        "HU" => 28,
        "IS" => 26,
        "IE" => 22,
        "IL" => 23,
        "IT" => 27,
        "JO" => 30,
        "KZ" => 20,
        "KW" => 30,
        "LV" => 21,
        "LB" => 28,
        "LI" => 21,
        "LT" => 20,
        "LU" => 20,
        "MT" => 31,
        "MR" => 27,
        "MU" => 30,
        "MD" => 24,
        "MC" => 27,
        "ME" => 22,
        "NL" => 18,
        "NO" => 15,
        "PK" => 24,
        "PL" => 28,
        "PT" => 25,
        "QA" => 29,
        "RO" => 24,
        "SM" => 27,
        "SA" => 24,
        "RS" => 22,
        "SK" => 24,
        "SI" => 19,
        "ES" => 24,
        "SE" => 24,
        "CH" => 21,
        "TN" => 24,
        "TR" => 26,
        "AE" => 23,
        "GB" => 22,
        _ => return None,
    })
}

fn iban_checksum(compact: &str) -> bool {
    let (head, tail) = compact.split_at(4);
    let mut remainder = 0u32;
    for byte in tail.bytes().chain(head.bytes()) {
        let value = match byte {
            b'0'..=b'9' => byte - b'0',
            b'A'..=b'Z' => byte - b'A' + 10,
            _ => return false,
        } as u32;
        if value >= 10 {
            remainder = (remainder * 10 + value / 10) % 97;
        }
        remainder = (remainder * 10 + value % 10) % 97;
    }
    remainder == 1
}

#[cfg(test)]
mod tests {
    use super::{redact, PersonalData};
    use crate::detectors::{Detector, Finding};

    fn spans(text: &str) -> Vec<(usize, usize)> {
        PersonalData
            .scan(text)
            .into_iter()
            .map(|finding| (finding.start, finding.end))
            .collect()
    }

    #[test]
    fn masks_global_identifiers_and_leaves_ordinary_numbers() {
        assert_eq!(spans("fale com maria.souza@email.com hoje").len(), 1);
        assert_eq!(spans("ligue +1 415 555 2671 hoje").len(), 1);
        assert_eq!(spans("card 4111 1111 1111 1111 fim").len(), 1);
        assert_eq!(spans("iban DE89 3704 0044 0532 0130 00.").len(), 1);
        assert_eq!(spans("CPF 529.982.247-25.").len(), 1);
        assert_eq!(spans("SSN 123-45-6789.").len(), 1);
        assert!(spans("pedido 1529982247259").is_empty());
        assert!(spans("CPF 111.111.111-11").is_empty());
        assert!(spans("SSN 000-12-3456").is_empty());
        assert!(spans("Qual o prazo de entrega?").is_empty());
    }

    #[test]
    fn masks_brazilian_phones_but_not_dates_cep_or_bare_numbers() {
        for (text, phone) in [
            ("ligue (11) 98765-4321 hoje", "(11) 98765-4321"),
            ("ligue (11)98765-4321 hoje", "(11)98765-4321"),
            ("ligue (21) 3456-7890 hoje", "(21) 3456-7890"),
            ("ligue (11) 987654321 hoje", "(11) 987654321"),
            ("ligue 11 98765-4321 hoje", "11 98765-4321"),
            ("ligue 98765-4321 hoje", "98765-4321"),
        ] {
            let found = spans(text);
            assert_eq!(found.len(), 1, "{text}");
            assert_eq!(&text[found[0].0..found[0].1], phone, "{text}");
        }
        for text in [
            "CEP 69000-000",
            "data 2026-11-22",
            "pedido 98765432199",
            "protocolo 1234-5678",
            "12345-67890",
            "nota 4,5 de 5",
        ] {
            assert!(spans(text).is_empty(), "{text}");
        }
    }

    #[test]
    fn redaction_removes_the_value() {
        let text = "email maria.souza@email.com";
        let findings: Vec<Finding> = PersonalData.scan(text);
        let hidden = redact(text, &findings);
        assert_eq!(hidden, "email [redacted]");
        assert!(!hidden.contains("maria"));
    }
}
