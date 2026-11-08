# Security policy

## Supported versions

Security fixes are provided for the latest release only.

## Reporting a vulnerability

Do **not** open a public issue for security problems. Instead, use GitHub's private
vulnerability reporting for this repository ("Security" tab → "Report a vulnerability"),
or contact the maintainer privately.

Include a description of the issue, steps to reproduce it and the affected version. We aim
to respond as soon as possible; fixes are announced in the changelog.

## Scope

In scope: flaws in how Nixport parses archives, handles paths, extracts settings or elevates
privileges.

Out of scope: the documented properties that archives are unsigned and unencrypted and that
`run` is not a sandbox. See [docs/security.md](docs/security.md).
