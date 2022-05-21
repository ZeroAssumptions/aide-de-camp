{
  description = "Aide De Camp Development Environment";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    andoriyu = {
      url = "github:andoriyu/flakes";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
        devshell.follows = "devshell";
      };
    };
    devshell = {
      url = "github:numtide/devshell/master";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    { self, nixpkgs, fenix, flake-utils, andoriyu, devshell, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        cwd = builtins.toString ./.;
        overlays = [ devshell.overlay fenix.overlay andoriyu.overlay ];
        pkgs = import nixpkgs { inherit system overlays; };
      in with pkgs; {
        devShell = clangStdenv.mkDerivation rec {
        name = "aide-de-camp-env";
        nativeBuildInputs = [
            (pkgs.fenix.complete.withComponents [
             "cargo"
             "clippy"
             "rust-src"
             "rustc"
             "rustfmt"
            ])
            rust-analyzer-nightly
            bacon
            cargo-cache
            cargo-deny
            cargo-diet
            cargo-sort
            cargo-sweep
            cargo-wipe
            cargo-outdated
            cmake
            gnumake
            openssl.dev
            pkgconfig
            rusty-man
            sqlx-cli
            atlas
            sqlite
            just
        ];
        PROJECT_ROOT = builtins.toString ./.;
        };
      });
}

