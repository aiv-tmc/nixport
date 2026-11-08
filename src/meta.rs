//! Archive manifest: the metadata stored in front of the payload.
//!
//! The manifest is a list of `key<TAB>value` lines. Values escape backslash,
//! newline, tab and carriage return. Keys that hold lists repeat once per item.

use crate::error::{fail, Error, Result};
use crate::log::{ja, js, object};

/// Placeholder in `Meta::exec` that is replaced by the user's arguments at launch.
pub const ARGS_MARKER: &str = "@@ARGS@@";

pub struct Meta {
    /// Identifier derived from the closure; names the cache and settings directories.
    pub id: String,
    /// Display name of the program.
    pub name: String,
    pub comment: String,
    /// Target platform in Nix notation, e.g. `x86_64-linux`.
    pub system: String,
    /// File extension of the embedded icon, empty when the archive has no icon.
    pub icon_ext: String,
    /// Creation time in seconds since the Unix epoch.
    pub created: u64,
    /// Number of store paths in the closure.
    pub paths: u64,
    /// Command line of the program; one element equals `ARGS_MARKER`.
    pub exec: Vec<String>,
    /// Store paths of the packaged program; the first one is installed by `install`.
    pub roots: Vec<String>,
    /// Settings paths relative to `$HOME` that travel with the archive.
    pub config_paths: Vec<String>,
}

/// The platform this binary runs on, in Nix notation.
pub fn current_system() -> String {
    format!("{}-linux", std::env::consts::ARCH)
}

/// Fails when the archive was built for a different platform.
pub fn check_system(meta: &Meta) -> Result<()> {
    let here = current_system();
    if meta.system == here {
        Ok(())
    } else {
        fail(format!("the archive targets {} but this machine is {here}", meta.system))
    }
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

fn parse_number(value: &str) -> Result<u64> {
    value.parse().map_err(|_| Error(format!("invalid number in manifest: {value}")))
}

impl Meta {
    /// Serialises the manifest to its on-disk text form.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = String::new();
        let mut put = |key: &str, value: &str| {
            out.push_str(key);
            out.push('\t');
            out.push_str(&escape(value));
            out.push('\n');
        };
        put("id", &self.id);
        put("name", &self.name);
        put("comment", &self.comment);
        put("system", &self.system);
        put("icon_ext", &self.icon_ext);
        put("created", &self.created.to_string());
        put("paths", &self.paths.to_string());
        for item in &self.exec {
            put("exec", item);
        }
        for item in &self.roots {
            put("root", item);
        }
        for item in &self.config_paths {
            put("config_path", item);
        }
        out.into_bytes()
    }

    /// Parses the on-disk text form. Unknown keys are ignored; required keys
    /// must be present.
    pub fn decode(bytes: &[u8]) -> Result<Meta> {
        let text = std::str::from_utf8(bytes).map_err(|_| Error("the manifest is not valid UTF-8".into()))?;
        let mut meta = Meta {
            id: String::new(),
            name: String::new(),
            comment: String::new(),
            system: String::new(),
            icon_ext: String::new(),
            created: 0,
            paths: 0,
            exec: Vec::new(),
            roots: Vec::new(),
            config_paths: Vec::new(),
        };
        for line in text.lines() {
            let (key, raw) = line
                .split_once('\t')
                .ok_or_else(|| Error(format!("malformed manifest line: {line}")))?;
            let value = unescape(raw);
            match key {
                "id" => meta.id = value,
                "name" => meta.name = value,
                "comment" => meta.comment = value,
                "system" => meta.system = value,
                "icon_ext" => meta.icon_ext = value,
                "created" => meta.created = parse_number(&value)?,
                "paths" => meta.paths = parse_number(&value)?,
                "exec" => meta.exec.push(value),
                "root" => meta.roots.push(value),
                "config_path" => meta.config_paths.push(value),
                _ => {}
            }
        }
        if meta.id.is_empty() || meta.name.is_empty() || meta.system.is_empty() || meta.exec.is_empty() || meta.roots.is_empty() {
            return fail("the manifest is missing required fields");
        }
        Ok(meta)
    }

    /// Renders the manifest as one JSON object for graphical shells.
    /// `icon` is the icon file extension, or null when there is no icon.
    pub fn to_json(&self) -> String {
        let icon = if self.icon_ext.is_empty() {
            "null".to_string()
        } else {
            js(&self.icon_ext)
        };
        object(&[
            ("id", js(&self.id)),
            ("name", js(&self.name)),
            ("comment", js(&self.comment)),
            ("system", js(&self.system)),
            ("icon", icon),
            ("created", self.created.to_string()),
            ("paths", self.paths.to_string()),
            ("exec", ja(&self.exec)),
            ("roots", ja(&self.roots)),
            ("config_paths", ja(&self.config_paths)),
        ])
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A fully populated manifest shared by the tests of several modules.
    pub fn sample() -> Meta {
        Meta {
            id: "0123456789abcdef".into(),
            name: "Hello\tWorld\nTwo".into(),
            comment: "back\\slash".into(),
            system: current_system(),
            icon_ext: "svg".into(),
            created: 1_700_000_000,
            paths: 42,
            exec: vec!["/nix/store/x/bin/hello".into(), ARGS_MARKER.into()],
            roots: vec!["/nix/store/x".into()],
            config_paths: vec![".config/hello".into()],
        }
    }

    #[test]
    fn manifest_survives_a_round_trip() {
        let original = sample();
        let decoded = Meta::decode(&original.encode()).unwrap();
        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.name, original.name);
        assert_eq!(decoded.comment, original.comment);
        assert_eq!(decoded.created, original.created);
        assert_eq!(decoded.exec, original.exec);
        assert_eq!(decoded.config_paths, original.config_paths);
    }

    #[test]
    fn decode_rejects_missing_fields() {
        assert!(Meta::decode(b"id\tabc\n").is_err());
    }

    #[test]
    fn decode_rejects_lines_without_a_tab() {
        assert!(Meta::decode(b"garbage\n").is_err());
    }

    #[test]
    fn json_marks_a_missing_icon_as_null() {
        let mut meta = sample();
        meta.icon_ext.clear();
        assert!(meta.to_json().contains("\"icon\":null"));
    }

    #[test]
    fn check_system_rejects_foreign_archives() {
        let mut meta = sample();
        meta.system = "nonexistent-os".into();
        assert!(check_system(&meta).is_err());
    }
}
