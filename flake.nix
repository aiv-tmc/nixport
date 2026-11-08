{
  description = "Nixport: portable Nix applications in a single file";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAll (pkgs: rec {
        nixport = pkgs.callPackage ./package.nix { };
        default = nixport;
      });

      overlays.default = final: _prev: { nixport = final.callPackage ./package.nix { }; };

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [ cargo rustc rustfmt clippy rust-analyzer nix bubblewrap zstd gnutar ];
        };
      });

      checks = forAll (pkgs: { build = self.packages.${pkgs.stdenv.hostPlatform.system}.nixport; });
    };
}
