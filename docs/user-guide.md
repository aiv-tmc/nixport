# User guide

This guide explains what Nixport does and how to use it day to day. For the exact syntax of every command, see the [CLI reference](cli-reference.md).

## 1. Concepts

**Closure.** A Nix program is not a single file: it needs libraries and tools stored under `/nix/store`. The program together with everything it needs, recursively, is its *closure*.

**`.nixport` file.** One compressed file that contains:

- a small header with the program's **name**, **icon** and launch command;
- optionally, the user's **settings** for that program;
- the complete **closure**.

**Run vs. install.**

| | `nixport run` | `nixport install` |
|---|---|---|
| Requires administrator rights | no | yes (polkit prompt) |
| Modifies the system `/nix/store` | no | yes |
| Program appears in the user profile and launcher | no | yes |
| Settings | applied from a portable copy; the machine's own settings remain untouched | restored into your home directory without overwriting existing files |
| Disk usage | a private copy under `~/.cache/nixport` | paths imported into `/nix/store` |

**Settings.** When packing, Nixport looks for the program's per-user configuration in `~/.config`, `~/.local/share`, `~/.local/state` and `~/.<name>`, and stores it in the file. Caches are never searched.

## 2. Creating a file

```sh
nixport pack TARGET [-o PATH]
```

`TARGET` can be:

| Target | Example | Notes |
|---|---|---|
| a `.desktop` entry | `/run/current-system/sw/share/applications/firefox.desktop` | best choice: provides name and icon |
| a command | `hello` | looked up in `PATH` |
| a store path | `/nix/store/…-hello-2.12` or `/nix/store/…/bin/hello` | a package directory needs `--exec` if it ships several programs |
| a flake reference | `nixpkgs#hello` | built with `nix build` first |

`-o PATH` may be a file name or a directory. If it is a directory (for example, the mount point of a USB stick), the file is named `<Program Name>.nixport`. Without `-o`, the file is created in the current directory.

Useful options:

- `--no-config` — include no settings.
- `--config-path .config/foo` — include an extra path relative to `$HOME`; repeatable.
- `--exec NAME` — select `bin/NAME` when a package contains several programs.
- `--level N` — zstd level from 1 to 19 (default: 10). Lower levels are faster; higher levels produce smaller files.

While packing, Nixport lists the settings it included. **Check that list**: settings can contain logins, cookies and tokens. Use `--no-config` when in doubt.

Packing writes to `<name>.nixport.part` and renames the file only after everything succeeds, so an interrupted run never leaves a broken archive under the final name.

## 3. Looking inside

```sh
nixport info App.nixport
nixport info App.nixport --icon-out /tmp/icon.svg
```

`info` prints a single JSON object containing the name, comment, platform, number of store paths, launch command and list of included settings. `--icon-out` writes the embedded icon (SVG or PNG) to the given path so you can display it.

## 4. Running without installing

```sh
nixport run App.nixport [arguments for the program]
```

On the **first** launch, the closure is unpacked into `~/.cache/nixport/<id>`. This can take a while for large programs; later launches start immediately. The program then starts inside a private mount namespace in which the unpacked closure is overlaid on `/nix/store`.

Things to know:

- Everything else is the real system: your home directory, `/tmp`, the display server and the network. `run` is *not* a sandbox.
- If the file contains settings, they are copied once into `~/.local/share/nixport/<id>/config` and mounted over their usual places in your home directory **only while the program runs**. The machine's own settings are not modified, and anything the program saves stays in that portable copy.
- `--no-config` starts the program without mounting the packed settings.
- Arguments after the file name are passed to the program. A leading `--` is removed: `nixport run App.nixport -- --version`.
- The program receives the environment variable `NIXPORT_ID`.
- The exit status is the program's own.

## 5. Installing

```sh
nixport install App.nixport
```

Nixport requests administrator rights through polkit, imports the closure into the system `/nix/store`, restores the packed settings into your home directory (existing files are never overwritten), and adds the program to your Nix profile, making its launcher entry and commands available. The installation applies to the user who started the command.

To remove an installed program, use the usual Nix tools, for example `nix profile remove <name>`. Imported store paths remain until the next garbage collection.

## 6. Disk space

| Location | Contents | How to free it |
|---|---|---|
| `~/.cache/nixport/<id>` | unpacked closure of a program started with `run` | `nixport clean [FILE]` |
| `~/.local/share/nixport/<id>` | portable settings of that program | delete the directory yourself; `clean` never removes it |

The unpacked copy contains the entire closure, including files already present in the system store; Nixport does not deduplicate them.

## 7. Updating a program

The archive ID is derived from its closure. Packing a newer version of a program produces a different ID, so the new archive starts with a fresh copy of the settings stored in it. Changes made in the portable settings of the old archive are not carried over automatically. To keep them, re-pack the program on the machine where those settings live, or copy the directory `~/.local/share/nixport/<old id>/config` manually.

## 8. Choosing a USB stick

- Use exFAT or ext4. FAT32 cannot store files larger than 4 GiB, and large programs exceed that easily.
- Packing time is dominated by compression; use `--level 3` for speed.

## 9. Compatibility

- The archive must be opened on the same architecture (`x86_64-linux` or `aarch64-linux`). Nixport rejects a mismatching file with a clear message.
- The program brings its own libraries, but graphics drivers and some system integration come from the host. For reliable results, use the same distribution, or a closely matching nixpkgs, on both machines.
