{ pkgs, lib, config, inputs, ... }: {
  packages = [
    pkgs.bacon
    pkgs.cargo-cache
    pkgs.cargo-deny
    pkgs.cargo-diet
    pkgs.cargo-nextest
    pkgs.cargo-outdated
    pkgs.cargo-sort
    pkgs.cargo-sweep
    pkgs.cargo-wipe
    pkgs.cargo-release
    pkgs.git-cliff
    pkgs.cmake
    pkgs.gnumake
    pkgs.pkg-config
    pkgs.openssl.dev
    pkgs.sqlite
    pkgs.sqlx-cli
    pkgs.just
  ];

  languages.rust = {
    enable = true;
    channel = "stable";
    components = ["rustc" "cargo" "clippy" "rustfmt" "rust-src"];
  };

  git-hooks.hooks = {
    rustfmt.enable = true;
    clippy = {
      enable = true;
      settings.denyWarnings = true;
    };
    cargo-check.enable = true;
  };

  enterShell = ''
    echo "aide-de-camp development environment"
  '';
}
