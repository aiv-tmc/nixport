# Nixport

Nixport packages a program from Nix into a single compressed `.nixport` file that you can carry on a USB drive and run on another device with the same distribution—without installing it into the system, and together with its settings. If desired, the same program can also be installed system-wide.

The file stores the program name and icon in its header, so a graphical frontend can display them without unpacking the archive.

> The project is written in Rust with no external dependencies. It requires `nix`, `bubblewrap` (version 0.11 or newer), `zstd`, and `tar`; the Nix package pulls them in automatically.

## How it works

- **pack** computes the program’s closure (`nix-store -qR`), exports it (`nix-store --export`), compresses it with zstd, and writes it to a single file together with the name, icon, and any discovered user settings.
- **run** imports the closure into a private store at `~/.cache/nixport/<id>` and launches the program in `bwrap`, with that store overlaid on top of `/nix/store`. The system is not modified. Settings are mounted over `~/.config/<app>` from a separate copy, so the host’s settings are left untouched.
- **install** imports the closure into the real `/nix/store` (via `pkexec`), restores the settings without overwriting existing files, and adds the program to the user profile.

Details: [docs/architecture.md](docs/architecture.md). File format: [docs/archive-format.md](docs/archive-format.md).

## Installation

With flakes:

```nix
{
  inputs.nixport.url = "path:/path/to/nixport";
  # ...
  environment.systemPackages = [ inputs.nixport.packages.${pkgs.system}.default ];
}
```

Without flakes:

```nix
environment.systemPackages = [ (pkgs.callPackage /path/to/nixport/package.nix { }) ];
```

`nixport install` requires `security.polkit.enable = true;`. You can add the project to your own package repository with the `scripts/install-to-repo.sh` script (see below).

## Quick start

```sh
# package a program from a launcher directly onto a USB drive
nixport pack /run/current-system/sw/share/applications/firefox.desktop -o /run/media/user/USB/

# view the name and icon
nixport info /run/media/user/USB/Firefox.nixport --icon-out /tmp/icon.png

# run without installing
nixport run /run/media/user/USB/Firefox.nixport

# install system-wide
nixport install /run/media/user/USB/Firefox.nixport
```

## Commands

| Command | Purpose |
|---|---|
| `pack TARGET [-o PATH] [--exec NAME] [--no-config] [--config-path REL]... [--level 1-19]` | Create a `.nixport`. `TARGET`: `.desktop` file, command, store path, or flake (`nixpkgs#hello`) |
| `run [--no-config] FILE [ARGS...]` | Run without installing |
| `install [--user NAME] FILE` | Install system-wide |
| `info FILE [--icon-out PATH]` | Print the manifest as JSON and save the icon |
| `clean [FILE]` | Remove unpacked programs from the cache |

The global `--json` flag (placed before the command) enables line-delimited JSON output for frontends: [docs/shell-integration.md](docs/shell-integration.md).

## Development

```sh
git init && git add -A        # flakes only see files tracked by git
nix develop                   # cargo, clippy, rustfmt, bwrap, zstd
cargo fmt                     # run once to format the code according to rustfmt.toml
cargo build
cargo clippy --all-targets -- -D warnings
cargo test
bash scripts/check-size.sh    # no source file longer than 300 lines
nix build .#nixport
bash scripts/smoke.sh ./result/bin/nixport
NIXPORT_SMOKE_RUN=1 bash scripts/smoke.sh ./result/bin/nixport   # includes a real run
```

Project rules: comments in English only and only about current behavior; source files no longer than 300 lines; no dead code or stubs.

## CI/CD

- `.github/workflows/ci.yml`: rustfmt, clippy, tests, file-size limit, package build, `nix flake check`, and a `pack` + `info` smoke test.
- `.github/workflows/release.yml`: on a `vX.Y.Z` tag (the version must match `Cargo.toml`), verifies the build and publishes a release with the source archive.

## Adding the project to a Nix package repository

```sh
bash scripts/install-to-repo.sh /path/to/repository            # pkgs/by-name/ni/nixport
bash scripts/install-to-repo.sh --layout flat /path/to/repository   # pkgs/nixport
bash scripts/install-to-repo.sh --dry-run --check /path/to/repository
```

The script copies `Cargo.toml`, `Cargo.lock`, `package.nix`, and `src/`; it does not modify anything else in the repository and prints instructions for registering the package.

## Limitations

- The archive is tied to a platform (`x86_64-linux`, etc.) and must be opened on the same distribution.
- Settings may include application tokens and sessions: the frontend should show the user the list from the `packing` event and allow disabling it (`--no-config`).
- FAT32 cannot store files larger than 4 GB; use exFAT or ext4.
- Troubleshooting: [docs/troubleshooting.md](docs/troubleshooting.md).
