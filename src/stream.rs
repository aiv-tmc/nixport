//! Child-process plumbing: shell-style pipelines and compressed-section readers.

use crate::error::{fail, Context, Error, Result};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::process::{ChildStdout, Command, Stdio};
use std::thread;

fn program_name(cmd: &Command) -> String {
    cmd.get_program().to_string_lossy().into_owned()
}

/// Runs `cmd` to completion and returns its trimmed stdout.
pub fn capture(cmd: &mut Command) -> Result<String> {
    let name = program_name(cmd);
    let out = cmd.output().context(&format!("cannot run {name}"))?;
    if !out.status.success() {
        return fail(format!("{name}: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Runs `stages` as `a | b | c > sink` and waits for all of them.
/// `tar` exiting with status 1 only means that a file changed while it was
/// being read, which is tolerated.
pub fn pipe(mut stages: Vec<Command>, sink: Stdio) -> Result<()> {
    let last = stages.len().saturating_sub(1);
    let mut sink = Some(sink);
    let mut previous: Option<ChildStdout> = None;
    let mut children = Vec::new();

    for (index, cmd) in stages.iter_mut().enumerate() {
        let name = program_name(cmd);
        if let Some(upstream) = previous.take() {
            cmd.stdin(Stdio::from(upstream));
        }
        match sink.take() {
            Some(target) if index == last => {
                cmd.stdout(target);
            }
            other => {
                sink = other;
                cmd.stdout(Stdio::piped());
            }
        }
        let mut child = cmd.spawn().context(&format!("cannot start {name}"))?;
        previous = child.stdout.take();
        children.push((name, child));
    }

    for (name, mut child) in children {
        let status = child.wait().context(&format!("cannot wait for {name}"))?;
        let tolerated = name == "tar" && status.code() == Some(1);
        if !status.success() && !tolerated {
            return fail(format!("{name} failed ({status})"));
        }
    }
    Ok(())
}

/// Feeds the zstd-compressed bytes `[offset, offset + length)` of `archive`
/// (to the end of the file when `length` is `None`) through `zstd -d` into the
/// stdin of `consumer`, then waits for both processes.
pub fn unpack(archive: &Path, offset: u64, length: Option<u64>, mut consumer: Command) -> Result<()> {
    let consumer_name = program_name(&consumer);
    let mut zstd = Command::new("zstd")
        .args(["-dc", "--long=27"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("cannot start zstd")?;
    let mut zstd_in = zstd.stdin.take().ok_or_else(|| Error("zstd stdin is unavailable".into()))?;
    let zstd_out = zstd.stdout.take().ok_or_else(|| Error("zstd stdout is unavailable".into()))?;

    // A dedicated thread copies the section into zstd so that the consumer
    // can read the decompressed stream concurrently. Dropping `zstd_in` at
    // the end of the thread signals end of input.
    let path = archive.to_path_buf();
    let reader = thread::spawn(move || -> io::Result<()> {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(offset))?;
        let copied = match length {
            Some(n) => io::copy(&mut file.take(n), &mut zstd_in),
            None => io::copy(&mut file, &mut zstd_in),
        };
        match copied {
            // The consumer may stop reading early; its exit status reports why.
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
            Err(e) => Err(e),
            Ok(_) => Ok(()),
        }
    });

    consumer.stdin(Stdio::from(zstd_out));
    let consumer_status = consumer.status();
    let zstd_status = zstd.wait().context("cannot wait for zstd")?;
    let read_result = reader.join().map_err(|_| Error("the archive reader thread panicked".into()))?;

    let consumer_status = consumer_status.context(&format!("cannot run {consumer_name}"))?;
    if !consumer_status.success() {
        return fail(format!("{consumer_name} failed ({consumer_status})"));
    }
    if !zstd_status.success() {
        return fail("the archive is corrupt (zstd failed)");
    }
    read_result.context("cannot read the archive")
}
