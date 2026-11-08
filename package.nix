# Nix derivation for the nixport package, following the nixpkgs by-name layout.
# The source is fetched from GitHub; dependencies are pinned via cargoHash.
{
  lib,
  rustPlatform,
  fetchFromGitHub,
  makeWrapper,
  nix,
  bubblewrap,
  zstd,
  gnutar,
  util-linux,
  coreutils,
  glibc,
}:

rustPlatform.buildRustPackage rec {
  pname = "nixport";
  version = "0.2.0";

  # Fetch the exact release tag from GitHub.
  # Replace "your-github-username" with the actual owner.
  src = fetchFromGitHub {
    owner = "aiv-tmc";
    repo = "nixport";
    rev = "v${version}";
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="; # nix-prefetch-github your-github-username nixport --rev v0.2.0
  };

  # The Cargo.lock file is committed upstream, so buildRustPackage can use it.
  # Because the crate has no external dependencies, the hash covers an empty
  # vendor directory; replace the fake hash after the first build.
  cargoHash = "sha256-BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB="; # lib.fakeHash

  nativeBuildInputs = [ makeWrapper ];

  # Wrap the binary so that the helper tools are found even under pkexec,
  # which resets PATH.
  postInstall = ''
    wrapProgram $out/bin/nixport \
      --prefix PATH : ${lib.makeBinPath [ nix bubblewrap zstd gnutar util-linux coreutils glibc.bin ]}
  '';

  meta = {
    description = "Portable Nix applications in a single file";
    homepage = "https://github.com/your-github-username/nixport";
    license = lib.licenses.mit; # Change to the license you actually use.
    maintainers = [ ]; # Add your nixpkgs maintainer handle when registered.
    mainProgram = "nixport";
    platforms = lib.platforms.linux;
  };
}
