# Security

## Trust model

A `.nixport` file contains executable programs. **Opening or installing one is as trusting as running any program you downloaded.** Nixport validates the structure of a file, not its origin.

| Property | Status |
|---|---|
| Authenticity (who made the file) | **not provided** — files are not signed |
| Confidentiality | **not provided** — files are not encrypted |
| Integrity against accidental corruption | detected: truncated or corrupt data makes zstd or `nix-store` fail |
| Integrity against tampering | **not provided** — there is no signature |
| Isolation of the running program | **not provided** — `run` is not a sandbox |

Treat archives from untrusted sources as untrusted executables. If you need to transfer a file over an untrusted channel, sign or encrypt it with a tool of your choice (for example `gpg` or `age`) and verify it before use.

## What `run` does and does not do

`run` changes only the `/nix/store` view seen by the program. Otherwise the program runs under your user account and can see your home directory, network, display and the entire host filesystem. A malicious program can read your files exactly like any other program you start. Overlay mounts are created inside an unprivileged user namespace by `bwrap`; no privileges are gained.

## What `install` does

`install` runs as root through polkit and imports the closure into `/nix/store`. The archive is not signed, so its authenticity is not verified. A hostile archive could therefore place arbitrary content in the store, and the program is then added to your profile. Install only files you trust.

## Settings

Settings may contain sessions, cookies, tokens and passwords. By default, `pack` includes everything it detects. Review the list (`config` in the `packing` event, `config_paths` in `info`), use `--no-config` for files you intend to share, and remember that the portable copy in `~/.local/share/nixport/<id>` is stored unencrypted on every machine that opens the file.

Restoration is conservative: `install` never overwrites existing files, and `run` never modifies the machine's own settings because the packed copy is mounted over them only while the program runs.

## Input validation

- Settings paths in a manifest must be relative and must not contain `..`; otherwise the file is rejected.
- Header size fields are capped, so a damaged header cannot trigger huge allocations.
- Extraction uses GNU `tar` with its default protections against absolute and parent paths.
- Archives for a different platform are refused.

## Reporting a vulnerability

See [SECURITY.md](../SECURITY.md).
