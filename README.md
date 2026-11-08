# Nixport

**Portable Nix applications in a single file.**

Nixport packs a program from the Nix store, its complete closure and, optionally, the user's
settings into one compressed `.nixport` file. Copy the file to a USB stick and open it on
another machine with Nixport: the program starts without installation and without changes
outside your home directory. The same file can install the program into the system if you
want to keep it.

The file also carries the program's **name and icon** in a small header, so a file manager
or desktop shell can display it without unpacking anything.

## Features

- **One file per program:** closure, settings, name and icon.
- **Run without installing:** the program starts in a private mount namespace; the host
  `/nix/store` and the rest of the system are left untouched.
- **Portable settings:** your configuration travels with the file and is applied without
  modifying the settings of the machine you run it on.
- **Optional installation** into the system with a single command.
- **Scriptable:** every command is non-interactive and can report progress as JSON lines,
  which makes it easy to build a graphical interface on top.
- **Small and dependency-free:** written in Rust using only the standard library;
  compression and archiving are delegated to `zstd` and `tar`.

## Status

Nixport is at version 0.2.0. It targets NixOS on `x86_64-linux` and `aarch64-linux`.
`run` relies on overlay mounts inside user namespaces, which depends on your kernel
configuration; see [Requirements](#requirements) and
[docs/troubleshooting.md](docs/troubleshooting.md).

## Requirements

| Requirement | Why |
|---|---|
| Linux with Nix (NixOS recommended) | the program closure comes from and runs on `/nix/store` |
| `bubblewrap` 0.11 or newer | overlay of the unpacked closure on `/nix/store` (`nixport run`) |
| Linux 5.11+, unprivileged user namespaces enabled | overlay mounts without root |
| `zstd`, `tar`, `coreutils`, `util-linux`, `glibc` (`getent`) | compression, archiving, user switching |
| polkit | privilege elevation for `nixport install` |

The Nix package wraps the binary so that all helper tools are found automatically.

Open an archive on the same CPU architecture it was created on. For predictable graphics and
driver behaviour, use the same distribution (or at least a similar nixpkgs) on both machines.

## Installation

With flakes:

```nix
{
  inputs.nixport.url = "github:aiv-tmc/nixport";

  outputs = { nixpkgs, nixport, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      modules = [
        ({ pkgs, ... }: {
          environment.systemPackages = [ nixport.packages.${pkgs.system}.default ];
          security.polkit.enable = true; # needed by `nixport install`
        })
      ];
    };
  };
}
```

Without flakes, from a checkout:

```nix
environment.systemPackages = [ (pkgs.callPackage /path/to/nixport/package.nix { }) ];
```

Try it without installing: `nix run github:aiv-tmc/nixport -- --help`.

## Quick start

```sh
# 1. Pack a program straight onto a USB stick.
nixport pack /run/current-system/sw/share/applications/firefox.desktop -o /run/media/me/USB/

# 2. Inspect the contents (name, icon, size, included settings).
nixport info /run/media/me/USB/Firefox.nixport --icon-out /tmp/icon.svg

# 3. On any machine with Nixport: run it without installing.
nixport run /run/media/me/USB/Firefox.nixport

# 4. Or install it permanently (asks for administrator rights).
nixport install /run/media/me/USB/Firefox.nixport
```

`pack` also accepts a command, a store path or a flake reference:

```sh
nixport pack hello -o .
nixport pack nixpkgs#hello -o .
nixport pack /nix/store/…-hello-2.12 -o .
```

## Commands

| Command | Purpose |
|---|---|
| `pack TARGET` | create a `.nixport` file |
| `run FILE [ARGS…]` | start the packed program without installing it |
| `install FILE` | install the packed program into the system |
| `info FILE` | print the manifest as JSON, optionally extract the icon |
| `clean [FILE]` | delete unpacked programs from the cache |

Put `--json` before the command to get machine-readable progress and errors.
Full reference: [docs/cli-reference.md](docs/cli-reference.md).

## Documentation

| Document | Audience | Contents |
|---|---|---|
| [User guide](docs/user-guide.md) | everyone | concepts, workflows, settings, disk space, updating |
| [CLI reference](docs/cli-reference.md) | everyone | every command, option, exit code, file and variable |
| [How it works](docs/how-it-works.md) | curious users, contributors | pipeline of each command, on-disk layout, design decisions |
| [Archive format](docs/archive-format.md) | tool authors | byte-level format of `.nixport` and its manifest |
| [Shell integration](docs/shell-integration.md) | desktop-shell developers | JSON events, drag-and-drop flow, examples |
| [Security](docs/security.md) | everyone | trust model, what is and is not protected |
| [Troubleshooting](docs/troubleshooting.md) | everyone | common errors and fixes, FAQ |
| [Development](docs/development.md) | contributors | building, testing, conventions, releases |
| [Publishing](docs/publishing.md) | maintainers | GitHub and nixpkgs submission notes |

Русская версия обзора: [README.ru.md](README.ru.md).

## Limitations

- Archives are tied to a CPU architecture and to Linux.
- `run` is not a security sandbox: the program sees your home directory and the whole host.
- Settings may contain secrets (sessions, tokens). Review them before sharing a file.
- Archives are neither signed nor encrypted.
- FAT32 cannot hold files larger than 4 GiB; use exFAT or ext4 on the stick.
- Programs whose desktop entry starts with a wrapper such as `env VAR=… app` are not
  recognised; pack the program directly instead.

## Contributing and security

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

## License

See the `LICENSE` file in the repository.
