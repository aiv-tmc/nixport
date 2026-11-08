{ lib, rustPlatform, makeWrapper, nix, bubblewrap, zstd, gnutar, util-linux, coreutils, glibc }:

rustPlatform.buildRustPackage {
  pname = "nixport";
  version = "0.2.0";

  # Local sources: the flake and CI build exactly what is checked out, with
  # no network access and no dependency on a published tag.
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [ ./Cargo.toml ./Cargo.lock ./src ];
  };
  cargoLock.lockFile = ./Cargo.lock;
  nativeBuildInputs = [ makeWrapper ];

  # The tools below are started as child processes; the wrapper guarantees
  # they are found even under pkexec, which resets PATH.
  postInstall = ''
    wrapProgram $out/bin/nixport \
      --prefix PATH : ${lib.makeBinPath [ nix bubblewrap zstd gnutar util-linux coreutils glibc.bin ]}
  '';

  meta = {
    description = "Portable Nix applications in a single file";
    mainProgram = "nixport";
    platforms = lib.platforms.linux;
  };
}
