# Development

## Getting the sources

```sh
git clone https://github.com/aiv-tmc/nixport
cd nixport
nix develop          # cargo, rustc, clippy, rustfmt, rust-analyzer, nix, bubblewrap, zstd, tar
```

Without Nix, you need a recent stable Rust toolchain and the runtime tools listed in the README. Flakes only see files tracked by git, so run `git add` on new files before `nix build`.

## Everyday commands

```sh
cargo build                                   # debug build
cargo fmt                                     # format (settings in rustfmt.toml)
cargo clippy --all-targets -- -D warnings     # lints must be clean
cargo test                                    # unit tests
nix build .#nixport -L                        # build the packaged program
./result/bin/nixport --version
```

Quick end-to-end check of a build:

```sh
nixport pack nixpkgs#hello --no-config -o /tmp/hello.nixport
nixport info /tmp/hello.nixport
nixport run /tmp/hello.nixport -- --version     # needs bubblewrap with overlay support
nixport clean /tmp/hello.nixport
```

## Project rules

1. **No dependencies.** The crate uses only `std`. Everything else is delegated to external tools started as child processes.
2. **Comments are in English** and describe what the code does *now*. They do not mention earlier versions or history.
3. **Source files stay short**: about 300 lines at most. Check with `wc -l src/*.rs` and split a module before it grows past the limit.
4. **No dead code and no stubs.** Everything that exists is used; unfinished features are not merged.
5. **Errors are messages for people.** Use `fail(...)` and `.context(...)` so that a failure states what was being attempted.
6. **Tests live next to the code** in `#[cfg(test)] mod tests`.

## Layout

See [How it works](how-it-works.md#source-layout) for the responsibility of each source file. The rest of the repository:

| Path | Contents |
|---|---|
| `Cargo.toml`, `Cargo.lock` | crate definition (no third-party packages) |
| `package.nix` | package built from the local checkout; used by the flake |
| `flake.nix` | package, overlay, development shell, checks |
| `nixpkgs/package.nix` | recipe prepared for submission to nixpkgs |
| `docs/` | documentation |
| `.github/workflows/` | CI and release automation |

## Adding a command

1. Add an `Action` variant and a `parse_<name>` function in `cli.rs`, and document it in `USAGE`.
2. Implement it in its own module (or in `inspect.rs` for a small read-only command) and dispatch it from `main.rs`.
3. Emit progress with `log::event` and fail with `fail`/`Error`, so that `--json` mode works without extra code.
4. Add unit tests for pure logic (parsing, path handling) and update the [CLI reference](cli-reference.md) and [Shell integration](shell-integration.md).

## Tests

Unit tests cover the manifest codec, the container header, store-path and desktop-entry parsing, JSON rendering, icon ranking, path validation and argument parsing. The paths that require Nix, `bwrap` or privileges (`pack`, `run`, `install`) are verified by running the end-to-end check above on a real system.

## Continuous integration and releases

The workflows in `.github/workflows/` check formatting, lints and tests, and build the Nix package. Pushing a tag `vX.Y.Z` (matching the version in `Cargo.toml`) publishes a release.

To cut a release:

1. Update `version` in `Cargo.toml` and in `package.nix`, and add a section to `CHANGELOG.md`.
2. Run `cargo build` (refreshes `Cargo.lock`) and commit.
3. Run `git tag -a vX.Y.Z -m "Nixport X.Y.Z" && git push origin vX.Y.Z`.

Submission to nixpkgs is described in [publishing.md](publishing.md).
