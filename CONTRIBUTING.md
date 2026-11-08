# Contributing

Thank you for your interest in Nixport. Bug reports, ideas and patches are welcome.

## Reporting problems

Open an issue and include:

- the command you ran, with `--json` output if relevant;
- the full error text;
- `nixport --version`;
- your distribution and kernel version (`uname -r`);
- `bwrap --version`.

For security problems, follow [SECURITY.md](SECURITY.md) instead.

## Sending changes

1. Fork the repository and create a branch.
2. Follow the rules in [docs/development.md](docs/development.md): no dependencies, English
   comments describing current behaviour, files of about 300 lines or fewer, and no dead
   code.
3. Run `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test`.
4. Update the documentation and `CHANGELOG.md` whenever behaviour changes.
5. Open a pull request describing what changed and why.

Keep pull requests small and focused; they are easier to review.
