# Troubleshooting and FAQ

This page lists common errors and answers frequently asked questions.

## Errors

**`the installed bubblewrap has no overlay support; version 0.11 or newer is required`**  
Update nixpkgs so that `bubblewrap` is at least version 0.11. The Nix package of Nixport already places a suitable `bwrap` on `PATH`; this error appears when you run a build that does not.

**`bwrap: … Operation not permitted` or `Creating new namespace failed`**  
Unprivileged user namespaces are disabled. On NixOS with a hardened kernel, set `security.allowUserNamespaces = true;` (and, on some kernels, the sysctl `kernel.unprivileged_userns_clone = 1`). Overlay mounts inside user namespaces require Linux 5.11 or newer.

**`bwrap: … overlay` / `mount` failures**  
The kernel or a security module forbids overlay mounts in user namespaces. Check `dmesg`, the kernel version and your security-module configuration.

**`the archive targets X but this machine is Y`**  
The file was created for a different CPU architecture. Re-pack it on a matching machine.

**`… is not a Nixport archive`**  
The file is not a `.nixport` file, or it is damaged or incomplete (for example, a copy to the stick was interrupted). Copy it again and compare checksums.

**`FAT file systems cannot hold files above 4 GiB`**  
Format the stick as exFAT or ext4, or pack a smaller program.

**`… is not located in /nix/store`**  
The target does not lead into the Nix store. Pack programs installed through Nix.

**`the desktop entry has no Exec key` / program not found**  
Point `pack` at a real `.desktop` file, or pass the command name or store path directly.

**`… contains several programs (…); choose one with --exec`**  
The package ships more than one binary. Pass `--exec NAME`.

**`the program was imported into /nix/store but could not be added to the user profile`**  
Both `nix profile install` and `nix-env -i` failed. The files are already in the store; add the program manually with the tool that manages your profile, for example `nix profile install /nix/store/<path>` or `nix-env -i /nix/store/<path>`. A profile managed by `nix-env` cannot be modified with `nix profile`, and vice versa.

**`--user is required when running as root`**  
When you run `install` directly as root, specify whose profile to use: `--user NAME`.

**`cannot start pkexec`**  
Install polkit (`security.polkit.enable = true;`) and make sure a polkit agent is running in your session.

## FAQ

**Does the target machine need the same distribution?**  
No. The program brings its own libraries, so package versions do not need to match. Graphics drivers and other system integration come from the host, however, so using the same distribution (or a closely matching nixpkgs) gives the most reliable results. The CPU architecture must match.

**Does it work on other Linux distributions with Nix installed?**  
Nixport is developed for NixOS. Other distributions with a working Nix installation may work but are not tested.

**Why is the first start slow?**  
The closure is unpacked into `~/.cache/nixport/<id>` once. Later starts reuse it.

**Where did my settings go?**  
Packed settings are copied to `~/.local/share/nixport/<id>/config` and mounted over their usual locations while the program runs. After `install`, they are placed in your normal home directory.

**I changed settings in the portable copy. Can I put them back into the file?**  
Re-pack the program on that machine with the settings you want. The file is not modified in place.

**How do I remove everything Nixport created?**  
`nixport clean` removes unpacked programs. Delete `~/.local/share/nixport` to remove portable settings. Installed programs are removed with `nix profile remove`.

**Is the file encrypted or signed?**  
No. See [Security](security.md).

**Can I run several different programs at the same time?**  
Yes; each `run` uses its own mount namespace.

**`cargo fmt --check` fails for contributors**  
Run `cargo fmt` and commit the result; see [Development](development.md).
