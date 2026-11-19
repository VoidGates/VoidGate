//! Turns an OpenAI-style chat body into a forward or a local refusal.
//!
//! shortcut: `messages[].content` must be a string. Multimodal content arrays
//! are rejected until the proxy can scan every part.

use serde_json::{json, Value};

use crate::{
    domain::detectors::{personal_data, Detector},
    error::AppError,
    services::chat::{action_of, inspect, MAX_CONTENT_BYTES},
};

const REFUSAL: &str = "I can't help with that request.";

pub(crate) enum Gate {
    Block,
    Forward(String),
}

pub(crate) fn gate(raw: &str, detectors: &[Box<dyn Detector>]) -> Result<Gate, AppError> {
    let mut value: Value = serde_json::from_str(raw).map_err(|error| {
        if error.is_syntax() {
            AppError::InvalidJson
        } else {
            AppError::InvalidBody
        }
    })?;
    let Some(messages) = value.get_mut("messages").and_then(Value::as_array_mut) else {
        return Err(AppError::InvalidBody);
    };
    if messages.is_empty() {
        return Err(AppError::InvalidBody);
    }

    let mut block = false;
    let mut masked: Vec<(usize, String)> = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        let Some(content) = message.get("content").and_then(Value::as_str) else {
            return Err(AppError::InvalidBody);
        };
        if content.len() > MAX_CONTENT_BYTES {
            return Err(AppError::PayloadTooLarge);
        }
        let findings = inspect(detectors, content);
        match action_of(&findings) {
            "block" => block = true,
            "mask" => masked.push((index, personal_data::redact(content, &findings))),
            _ => {}
        }
    }
    if block {
        return Ok(Gate::Block);
    }
    if masked.is_empty() {
        return Ok(Gate::Forward(raw.to_owned()));
    }
    for (index, content) in masked {
        messages[index]["content"] = Value::String(content);
    }
    serde_json::to_string(&value)
        .map(Gate::Forward)
        .map_err(|_| AppError::InvalidBody)
}

/// Removes personal data from an upstream completion. A blocked assistant
/// message is replaced with a generic refusal.
pub(crate) fn scrub(raw: &str, detectors: &[Box<dyn Detector>]) -> Result<String, AppError> {
    let mut value: Value = serde_json::from_str(raw).map_err(|_| AppError::Unavailable)?;
    if let Some(choices) = value.get_mut("choices").and_then(Value::as_array_mut) {
        for choice in choices {
            let Some(content) = choice.pointer_mut("/message/content") else {
                continue;
            };
            let Some(text) = content.as_str() else {
                continue;
            };
            let owned = text.to_owned();
            let findings = inspect(detectors, &owned);
            match action_of(&findings) {
                "block" => *content = json!(REFUSAL),
                "mask" => *content = json!(personal_data::redact(&owned, &findings)),
                _ => {}
            }
        }
    }
    serde_json::to_string(&value).map_err(|_| AppError::Unavailable)
}

pub(crate) fn refusal_body() -> &'static str {
    r#"{"choices":[{"finish_reason":"content_filter","message":{"role":"assistant","content":"I can't help with that request."}}]}"#
}
