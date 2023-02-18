{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk.url = "github:nix-community/naersk/master";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
      in
      rec {
        # `nix build`
        packages.ansine = pkgs.callPackage ./. { inherit naersk-lib; };
        packages.default = packages.ansine;
        overlays.ansine = final: prev: { inherit (packages) ansine; };
        overlays.default = self.overlays.ansine;
        nixosModules.ansine = { pkgs, ... }: {
          nixpkgs.overlays = [ self.overlays.default ];
          imports = [ ./module.nix ];
        };
        nixosModules.default = self.nixosModules.ansine;
        devShell = with pkgs; mkShell {
          buildInputs = [ cargo rustc rustfmt rustPackages.clippy ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
        formatter = pkgs.nixpkgs-fmt;
      });
}
