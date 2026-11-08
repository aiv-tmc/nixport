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
    hash = lib.fakeHash;
  };

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

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
