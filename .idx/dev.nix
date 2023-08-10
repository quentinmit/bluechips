{pkgs}: let
  rpkgs = pkgs.extend (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"));
in {
  # Use https://search.nixos.org/packages?channel=unstable to  find packages
  packages = with rpkgs; [
    #(rust-bin.fromRustupToolchainFile ../discobiker/frontend/rust-toolchain.toml)
    rust-bin.stable.latest.default
    #trunk
    stdenv.cc
    openssl
    openssl.dev
    pkg-config
    #google-cloud-sdk
  ];

  # sets environment variables in the workspace
  env = {
    # SOME_ENV_VAR = "hello";
    NIX_USER_CONF_FILES = pkgs.writeText "nix.conf" ''
      extra-experimental-features = nix-command flakes repl-flake
    '';
    ROCKET_LOG_LEVEL = "debug";
  };

  ide = {
    # search for the extension on https://open-vsx.org/ and use "publisher.id"

    extensions = [
      # "angular.ng-template"
    ];

    # preview configuration, identical to monospace.json
    previews = [
      {
        command = [
          "env"
          "ROCKET_PORT=$PORT"
          "ROCKET_ADDRESS=0.0.0.0"
          "cargo"
          "run"
        ];
        cwd = "bluechips-rs";

        manager = "web";
        id = "web";
      }
    ];
  };
}