//! Parsing of freedesktop `.desktop` entries.

use crate::error::{Context, Result};
use crate::meta::ARGS_MARKER;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Reads the `[Desktop Entry]` group of a desktop file.
pub fn parse_desktop(path: &Path) -> Result<HashMap<String, String>> {
    let text = fs::read_to_string(path).context(&format!("cannot read {}", path.display()))?;
    let mut keys = HashMap::new();
    let mut inside = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[Desktop Entry]";
        } else if inside && !line.starts_with('#') {
            if let Some((key, value)) = line.split_once('=') {
                keys.entry(key.trim().to_string()).or_insert_with(|| value.trim().to_string());
            }
        }
    }
    Ok(keys)
}

/// Splits an `Exec=` value into words, honouring double quotes and the
/// backslash escapes allowed inside them.
fn split_exec(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '\\' if quoted => {
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                }
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    words.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(current);
    }
    words
}

/// Converts an `Exec=` value to a command line: file and URL field codes
/// become `ARGS_MARKER`, all other field codes are dropped, `%%` becomes `%`.
pub fn exec_argv(line: &str) -> Vec<String> {
    split_exec(line)
        .into_iter()
        .filter_map(|word| match word.as_str() {
            "%f" | "%F" | "%u" | "%U" => Some(ARGS_MARKER.to_string()),
            code if code.len() == 2 && code.starts_with('%') && code != "%%" => None,
            _ => Some(word.replace("%%", "%")),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_field_codes_are_translated() {
        assert_eq!(exec_argv("firefox --new-window %U"), vec!["firefox", "--new-window", ARGS_MARKER]);
        assert_eq!(exec_argv("/bin/x \"a b\" %i 100%%"), vec!["/bin/x", "a b", "100%"]);
    }

    #[test]
    fn quoted_exec_words_keep_their_spaces() {
        assert_eq!(split_exec("prog \"one two\"  three"), vec!["prog", "one two", "three"]);
    }
}
