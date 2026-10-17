//! The JSON object in an answer of a model, and its texts.

use serde_json::Value;

/// The JSON object in an answer: the answer itself, or the first
/// object in it (a model may put it in a code block, or add words).
pub(super) fn json_object(answer: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(answer.trim())
        && value.is_object()
    {
        return Some(value);
    }

    let start = answer.find('{')?;
    let end = answer.rfind('}')?;

    serde_json::from_str::<Value>(answer.get(start..=end)?).ok().filter(Value::is_object)
}

/// The text of `name` in `value`, without spaces around it; empty when
/// it is missing or not a text.
pub(super) fn text_of(value: &Value, name: &str) -> String {
    value.get(name).and_then(Value::as_str).unwrap_or_default().trim().to_owned()
}
