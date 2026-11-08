# The `.nixport` format

All integers are unsigned 64-bit **little-endian**.

```text
offset    size            field
0         8               magic: "NIXPORT" followed by the byte 0x01
8         8               manifest length M
16        M               manifest (UTF-8 text)
16+M      8               icon length I
24+M      I               icon file bytes (SVG or PNG); I may be 0
24+M+I    8               settings length C
32+M+I    C               settings: tar archive compressed with zstd; C may be 0
32+M+I+C  rest of file    closure: `nix-store --export` stream compressed with zstd
```

Readers must reject files whose magic differs and should reject manifests or icons larger
than 64 MiB. Nixport enforces both limits.

## Why this order

The manifest and icon come first so that a file manager can read the name and icon from the
first few kilobytes. The settings precede the closure because their length is known only
after they are written: the writer reserves the length field, streams the settings, then
goes back and fills the field in. The closure is the last section and runs to the end of the
file, so it needs no length field and can be streamed.

## Manifest

A sequence of lines in the form `key<TAB>value<LF>`. In values, `\` is written as `\\`, line
feed as `\n`, tab as `\t` and carriage return as `\r`. Keys marked *repeated* appear once per
list item, in order.

| Key | Required | Description |
|---|---|---|
| `id` | yes | 16 hexadecimal digits derived from the closure; names the cache and settings directories |
| `name` | yes | display name |
| `comment` | no | one-line description |
| `system` | yes | platform in Nix notation, e.g. `x86_64-linux` |
| `icon_ext` | no | extension of the embedded icon (`svg`, `png`); empty when there is no icon |
| `created` | no | creation time, in seconds since the Unix epoch |
| `paths` | no | number of store paths in the closure |
| `exec` (repeated) | yes | words of the launch command; the word `@@ARGS@@` is replaced by the user's arguments (possibly none) |
| `root` (repeated) | yes | store paths of the packaged program; the first one is installed by `install` |
| `config_path` (repeated) | no | settings paths relative to `$HOME`, contained in the settings section |

Readers must ignore unknown keys. A line without a tab is an error.

## Settings section

A tar archive whose member names are the `config_path` values (relative to `$HOME`).
Extractors must not allow absolute names or `..` components. Nixport validates the manifest
paths and relies on GNU tar's default protections during extraction.

## Closure section

The output of `nix-store --export <paths…>`, compressed as a single zstd stream created with
`--long=27` (a 128 MiB window). Decompressors must use the same `--long=27` setting. The
decompressed stream can be passed to `nix-store --import` for either the system store or a
private store (`--store <dir>`).

## Compatibility

The final magic byte is the format version. A future incompatible layout will use a
different byte, so older readers fail cleanly instead of misreading the file.
