//! `nixport pack`: builds a `.nixport` archive from a program.

use crate::config;
use crate::container::Writer;
use crate::error::{Context, Error, Result};
use crate::icon;
use crate::log::{self, ja, js};
use crate::meta::{current_system, Meta};
use crate::paths;
use crate::resolve::{self, Target};
use crate::stream;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeSet;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Directory names that never travel with the settings: they only hold
/// caches and lock files that the program rebuilds on its own.
const SETTINGS_EXCLUDES: [&str; 5] = ["Cache", "GPUCache", "CachedData", "Code Cache", "*.lock"];

/// Parsed command-line options of `pack`.
pub struct Options {
    pub target: String,
    pub output: Option<PathBuf>,
    pub exec: Option<String>,
    pub no_config: bool,
    pub config_paths: Vec<String>,
    /// zstd compression level, 1 to 19.
    pub level: u32,
}

/// Identifier of a closure: a hash of its sorted store paths.
fn closure_id(closure: &[String]) -> String {
    let mut sorted = closure.to_vec();
    sorted.sort();
    let mut hasher = DefaultHasher::new();
    sorted.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Chooses the settings paths to embed: those detected for the program plus
/// the ones the user named, limited to paths that exist.
fn select_config(options: &Options, target: &Target) -> Result<Vec<String>> {
    if options.no_config {
        return Ok(Vec::new());
    }
    let program = Path::new(&target.argv[0]).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let names = vec![
        resolve::pname(&target.roots[0]),
        program,
        target.desktop.get("StartupWMClass").cloned().unwrap_or_default(),
        target.name.clone(),
        target.desktop_stem.clone().unwrap_or_default(),
    ];
    let mut selected: BTreeSet<String> = config::detect(&names)?.into_iter().collect();
    for extra in &options.config_paths {
        paths::safe_relative(extra)?;
        selected.insert(extra.clone());
    }
    let home = paths::home()?;
    Ok(selected.into_iter().filter(|p| home.join(p).exists()).collect())
}

/// Loads the icon named by the desktop entry. Returns the raw bytes and the
/// file extension, both empty when no icon is available.
fn load_icon(target: &Target, closure: &[String]) -> Result<(Vec<u8>, String)> {
    let Some(name) = target.desktop.get("Icon") else {
        return Ok((Vec::new(), String::new()));
    };
    let mut roots = closure.to_vec();
    roots.push("/run/current-system/sw".to_string());
    let Some(path) = icon::find(name, &roots) else {
        log::event(&format!("icon {name} was not found; the archive will have no icon"), &[]);
        return Ok((Vec::new(), String::new()));
    };
    let extension = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let bytes = fs::read(&path).context(&format!("cannot read {}", path.display()))?;
    Ok((bytes, extension))
}

/// Replaces characters that are unsafe in file names.
fn sanitize(name: &str) -> String {
    name.chars().map(|c| if c.is_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' }).collect()
}

/// Output file: the given path, or `<name>.nixport` inside the given directory
/// (the current directory by default).
fn output_path(output: &Option<PathBuf>, name: &str) -> PathBuf {
    let file_name = format!("{}.nixport", sanitize(name));
    match output {
        Some(path) if path.is_dir() => path.join(file_name),
        Some(path) => path.clone(),
        None => PathBuf::from(file_name),
    }
}

/// Warns that FAT file systems cannot hold large archives.
fn fat_hint(output: &Path) -> &'static str {
    let dir = output.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut probe = Command::new("stat");
    probe.args(["-f", "-c", "%T"]).arg(dir);
    match stream::capture(&mut probe).as_deref() {
        Ok("msdos") | Ok("vfat") => " (FAT file systems cannot hold files above 4 GiB; use exFAT or ext4)",
        _ => "",
    }
}

/// Writes the archive sections: manifest, icon, settings, closure.
fn write_parts(file: &Path, meta: &Meta, icon: &[u8], closure: &[String], level: u32) -> Result<()> {
    let mut writer = Writer::create(file, meta, icon)?;

    let config_pos = writer.begin_config()?;
    if !meta.config_paths.is_empty() {
        let mut tar = Command::new("tar");
        tar.arg("-cf").arg("-").arg("-C").arg(paths::home()?);
        for pattern in SETTINGS_EXCLUDES {
            tar.arg(format!("--exclude={pattern}"));
        }
        tar.arg("--").args(&meta.config_paths);
        let mut zstd = Command::new("zstd");
        zstd.args(["-T0", "-3"]);
        stream::pipe(vec![tar, zstd], writer.sink()?)?;
    }
    writer.end_config(config_pos)?;

    let mut export = Command::new("nix-store");
    export.arg("--export").args(closure);
    let mut zstd = Command::new("zstd");
    zstd.args(["-T0", "--long=27"]).arg(format!("-{level}"));
    stream::pipe(vec![export, zstd], writer.sink()?)
}

/// Writes to `<output>.part` and renames on success, so an interrupted run
/// never leaves a half-written archive under the final name.
fn write_archive(output: &Path, meta: &Meta, icon: &[u8], closure: &[String], level: u32) -> Result<()> {
    let partial = PathBuf::from(format!("{}.part", output.display()));
    match write_parts(&partial, meta, icon, closure, level) {
        Ok(()) => fs::rename(&partial, output).context(&format!("cannot create {}", output.display())),
        Err(e) => {
            fs::remove_file(&partial).ok();
            Err(Error(format!("{}{}", e.0, fat_hint(output))))
        }
    }
}

pub fn run(options: Options) -> Result<()> {
    let target = resolve::resolve(&options.target, options.exec.as_deref())?;
    log::event(&format!("computing the closure of {}", target.name), &[]);
    let mut query = Command::new("nix-store");
    query.arg("-qR").args(&target.roots);
    let closure: Vec<String> = stream::capture(&mut query)?.lines().map(str::to_string).collect();

    let config_paths = select_config(&options, &target)?;
    let (icon_bytes, icon_ext) = load_icon(&target, &closure)?;
    let output = output_path(&options.output, &target.name);
    let meta = Meta {
        id: closure_id(&closure),
        name: target.name.clone(),
        comment: target.desktop.get("Comment").cloned().unwrap_or_default(),
        system: current_system(),
        icon_ext,
        created: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        paths: closure.len() as u64,
        exec: target.argv.clone(),
        roots: target.roots.clone(),
        config_paths,
    };

    log::event(
        "packing",
        &[("paths", closure.len().to_string()), ("config", ja(&meta.config_paths)), ("output", js(&output.display().to_string()))],
    );
    write_archive(&output, &meta, &icon_bytes, &closure, options.level)?;
    let size = fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
    log::event("done", &[("output", js(&output.display().to_string())), ("size", size.to_string()), ("id", js(&meta.id))]);
    Ok(())
}
