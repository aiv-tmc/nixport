# CLI reference

```text
nixport [--json] <command> [options]
```

## Global options

| Option | Meaning |
|---|---|
| `--json` | Report progress on stdout and errors on stderr as JSON lines. Must come **before** the command. |
| `-h`, `--help`, `help` | Print the usage text. Also printed when no command is given. |
| `-V`, `--version` | Print `nixport <version>`. |

## Exit status

| Code | Meaning |
|---|---|
| `0` | success |
| `1` | the command failed while running (missing file, failed import, …) |
| `2` | the command line could not be understood |

`nixport run` replaces itself with the program, so its exit status is that of the program.

## `pack`

```text
nixport pack TARGET [-o PATH] [--exec NAME] [--no-config] [--config-path REL]... [--level N]
```

Creates a `.nixport` file from `TARGET`.

| Option | Meaning |
|---|---|
| `TARGET` | A `.desktop` file, a command name, a path in `/nix/store`, a package directory, or a flake reference containing `#`. |
| `-o`, `--output PATH` | Output file, or an existing directory in which `<Name>.nixport` is created. Default: `<Name>.nixport` in the current directory. |
| `--exec NAME` | Use `bin/NAME` from the package. Needed when a package directory or flake output ships several programs and none is named after the package. |
| `--no-config` | Include no settings at all. |
| `--config-path REL` | Also include `$HOME/REL`. Repeatable. Absolute paths and `..` are rejected. Paths that do not exist are skipped. |
| `--level N` | zstd compression level for the program data, 1–19. Default `10`. |

How the program is located:

1. For a `.desktop` file, the `Exec=` line is split into words. `%f %F %u %U` mark where
   launch arguments are inserted; other field codes (`%i`, `%c`, …) are dropped; `%%` becomes
   `%`. The first word must lead into `/nix/store` (symlinks are followed one hop at a time).
   The entry's `Name`, `Comment`, `Icon` and `StartupWMClass` are used.
2. Otherwise the target is resolved to a program under `/nix/store`. If the owning package
   ships a `share/applications/*.desktop` file, the first one alphabetically supplies the
   metadata.
3. If the command line has no argument placeholder, arguments are appended at the end.

Settings are detected by case-insensitive matching of the names of entries in `~/.config`,
`~/.local/share` and `~/.local/state`, and `~/.<name>`, against the package name, the program
file name, `StartupWMClass`, the display name and the desktop-entry file name. The following
patterns are excluded from the settings archive: `Cache`, `GPUCache`, `CachedData`,
`Code Cache`, `*.lock`.

The icon named by `Icon=` is taken from the closure, `share/pixmaps`, or
`/run/current-system/sw/share/icons`; SVG is preferred, otherwise the largest PNG. If no
icon is found, a message is printed and the archive is created without an icon.

Progress events: `computing the closure of <name>`, `packing` (with `paths`, `config`,
`output`), `done` (with `output`, `size`, `id`).

## `run`

```text
nixport run [--no-config] FILE [ARGS...]
```

Starts the packed program without installing it.

| Option | Meaning |
|---|---|
| `--no-config` | Do not restore or mount the packed settings. |
| `ARGS` | Passed to the program. Everything after `FILE` belongs to the program; a leading `--` is removed. |

Steps: check the platform, check that `bwrap` supports overlays, unpack the closure into the
cache if it is not there yet, restore the settings if needed, then execute `bwrap`. The
program receives `NIXPORT_ID=<id>`.

In `--json` mode, events (for example `first launch: unpacking the program`) are printed
**before** the program starts; after that, stdout belongs to the program.

## `install`

```text
nixport install [--user NAME] FILE
```

Installs the program into the system and into the profile of `NAME`.

| Option | Meaning |
|---|---|
| `--user NAME` | The user whose profile and home directory are used. Default: the user who runs the command. Required when started directly as root. |

When not run as root, the command re-executes itself through `pkexec`. As root it imports the
closure into `/nix/store`, restores settings into the user's home with `tar --skip-old-files`,
and runs `nix profile install` for the first root of the archive; if that fails, it falls back
to `nix-env -i`.

Events: `importing the program into /nix/store`, `restoring settings; existing files are
kept`, `adding the program to the user profile`, `<name> is installed`.

## `info`

```text
nixport info FILE [--icon-out PATH]
```

Prints the manifest as a single JSON object on stdout (always JSON, with or without `--json`):

```json
{"id":"0a1b2c3d4e5f6071","name":"Firefox","comment":"…","system":"x86_64-linux","icon":"svg",
 "created":1700000000,"paths":412,"exec":["/nix/store/…/bin/firefox","@@ARGS@@"],
 "roots":["/nix/store/…"],"config_paths":[".config/mozilla"]}
```

`icon` is the icon's file extension or `null`. The placeholder `@@ARGS@@` marks where launch
arguments are inserted. `--icon-out` writes the icon bytes to `PATH`; it fails if the archive
has no icon. Only the header is read, so `info` is fast regardless of archive size.

## `clean`

```text
nixport clean [FILE]
```

Removes unpacked programs from `~/.cache/nixport`: either only the one belonging to `FILE`, or
all of them. Portable settings are never removed.

## Environment

| Variable | Used for |
|---|---|
| `HOME` | must be an absolute path; base for settings and default cache/data locations |
| `XDG_CACHE_HOME` | cache location (`$XDG_CACHE_HOME/nixport`), default `~/.cache` |
| `XDG_DATA_HOME` | portable settings (`$XDG_DATA_HOME/nixport`), default `~/.local/share` |
| `PATH` | lookup of commands given as `TARGET` |
| `NIXPORT_ID` | **set** by `run` for the started program |

## Files

| Path | Contents |
|---|---|
| `~/.cache/nixport/<id>/nix/store` | unpacked closure used for the overlay |
| `~/.cache/nixport/<id>/.ready` | marker: the import finished |
| `~/.local/share/nixport/<id>/config` | portable settings of an archive |
| `~/.local/share/nixport/<id>/.restored` | marker: settings were extracted |
