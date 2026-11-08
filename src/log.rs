//! Progress and error reporting.
//!
//! In human mode messages go to stderr as plain text. In JSON mode every
//! message is one JSON object per line, so a graphical shell can parse it.

use std::sync::atomic::{AtomicBool, Ordering};

static JSON: AtomicBool = AtomicBool::new(false);

/// Switches between human-readable and JSON-lines output.
pub fn set_json(on: bool) {
    JSON.store(on, Ordering::Relaxed);
}

/// Returns true when messages must be emitted as JSON.
pub fn is_json() -> bool {
    JSON.load(Ordering::Relaxed)
}

/// Renders `s` as a quoted and escaped JSON string.
pub fn js(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Renders a list of strings as a JSON array.
pub fn ja(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|s| js(s)).collect();
    format!("[{}]", parts.join(","))
}

/// Renders a JSON object; every value must already be valid JSON text.
pub fn object(fields: &[(&str, String)]) -> String {
    let parts: Vec<String> = fields.iter().map(|(k, v)| format!("{}:{}", js(k), v)).collect();
    format!("{{{}}}", parts.join(","))
}

/// Reports progress. `fields` carries extra machine-readable data that is
/// only shown in JSON mode.
pub fn event(msg: &str, fields: &[(&str, String)]) {
    if is_json() {
        let mut all = vec![("msg", js(msg))];
        all.extend(fields.iter().cloned());
        println!("{}", object(&all));
    } else {
        eprintln!("nixport: {msg}");
    }
}

/// Reports a fatal error on stderr.
pub fn error(msg: &str) {
    if is_json() {
        eprintln!("{}", object(&[("error", js(msg))]));
    } else {
        eprintln!("nixport: error: {msg}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_are_escaped() {
        assert_eq!(js("a\"b\n"), "\"a\\\"b\\n\"");
        assert_eq!(js("\u{1}"), "\"\\u0001\"");
    }

    #[test]
    fn objects_and_arrays_are_composed() {
        let list = ja(&["x".to_string(), "y".to_string()]);
        assert_eq!(
            object(&[("list", list), ("n", "3".to_string())]),
            "{\"list\":[\"x\",\"y\"],\"n\":3}"
        );
    }
}
