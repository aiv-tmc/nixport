# How it works

## Source layout

| File | Responsibility |
|---|---|
| `main.rs` | entry point, command dispatch |
| `cli.rs` | argument parsing and usage text |
| `pack.rs` | `pack`: closure, settings, icon, writing the archive |
| `run.rs` | `run`: unpack into a private store, start via `bwrap` |
| `install.rs` | `install`: privilege elevation, import, profile |
| `inspect.rs` | `info` and `clean` |
| `resolve.rs` | turns a target into a program inside `/nix/store` |
| `desktop.rs` | parsing of `.desktop` files and `Exec=` lines |
| `icon.rs` | icon lookup |
| `config.rs` | detection of a program's per-user settings |
| `container.rs` | header and sections of the `.nixport` file |
| `meta.rs` | manifest encoding, decoding and JSON |
| `stream.rs` | child-process pipelines and compressed-section reading |
| `paths.rs` | cache/data directories, path validation, tree removal |
| `log.rs` | human and JSON output |
| `error.rs` | shared error type |

The crate has no dependencies. Heavy lifting is delegated to established tools that Nixport
starts as child processes: `nix-store`, `nix`, `tar`, `zstd`, `bwrap`, `runuser`, `pkexec`,
`getent`, `id`, `stat`, `chmod`.

## `pack`

1. **Resolve.** The target is resolved to a program path under `/nix/store`. Symlinks are
   followed one hop at a time until the path enters the store, which preserves the name of
   the final link (some programs behave differently depending on the name they are started
   as). The owning package(s) become the *roots* of the archive.
2. **Closure.** `nix-store -qR <roots>` lists every store path the program needs. A hash of
   the sorted list becomes the archive **id**.
3. **Settings and icon.** Per-user settings and the icon are located (see the
   [CLI reference](cli-reference.md#pack)).
4. **Write.** A `<file>.part` file is written in order: header (magic, manifest, icon), the
   settings as `tar | zstd -3`, then the closure as `nix-store --export | zstd --long=27`.
   Child processes write directly into the file, so nothing is buffered in memory or in
   temporary files. On success, the file is renamed to its final name.

## `run`

1. Verify the platform and that `bwrap` supports `--overlay-src`.
2. **Unpack once.** `nix-store --store ~/.cache/nixport/<id> --import` reads the
   decompressed export stream into a *chroot store* owned by the user, so no privileges are
   needed. A `.ready` marker is created only after a complete import; a failed import is
   removed.
3. **Settings.** On first use, the settings are extracted to
   `~/.local/share/nixport/<id>/config`. Each settings path is then bind-mounted over its
   place in `$HOME`.
4. **Start.** `bwrap` runs the program in a new mount namespace with:
   - `--bind / /` — the host root stays visible;
   - the settings bind mounts;
   - `--overlay-src /nix/store --overlay-src <cache>/nix/store --ro-overlay /nix/store` — an
     overlay whose lower layers are the host store and the unpacked closure.

Because the packed paths appear at their original `/nix/store/<hash>-name` locations, every
absolute path baked into the program's binaries keeps working without relocation or
patching. Because identical hashes mean identical contents, paths present in both layers are
interchangeable.

## `install`

Runs as root through `pkexec` on behalf of the invoking user. The closure is imported with
`nix-store --import` into the real store, settings are extracted into the user's home
directory with `--skip-old-files`, and the first root is added to the user's profile.

## Design decisions

- **Export stream instead of a file tree.** `nix-store --export` carries the metadata Nix
  needs (references, hashes), so the same payload serves both `run` (private store) and
  `install` (system store).
- **Overlay instead of relocating paths.** Rewriting store paths would change hashes and
  break programs; an overlay leaves the program unchanged.
- **Header first.** Name and icon precede the bulky payload, so inspecting a multi-gigabyte
  file is instant.
- **External tools instead of crates.** Fewer moving parts, an easily auditable dependency
  list, and a lockfile with no third-party packages.
- **Portable settings via bind mounts.** The machine's own settings are never modified, and
  the program can keep saving into its portable copy.

## Known limitations

- The unpacked cache is a full copy of the closure, including paths the host already has.
- Settings are restored once per archive id and are not refreshed from later archives.
- Programs started through wrapper commands in their `Exec=` line are not recognised.
- Only the first root of an archive is installed by `install`.
