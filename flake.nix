{
  description = "py-profinet";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }: let
    overlay = (final: prev: {
      bluechips-rs = final.rustPlatform.buildRustPackage rec {
        pname = "bluechips-rs";
        version = "0.1.0";

        src = ./bluechips-rs;

        cargoLock = {
          lockFile = ./bluechips-rs/Cargo.lock;
          outputHashes = {
            "askama-0.12.1" = "sha256-tjkjf/2vSZQ2ljQakhsWFkGhJN/32JmRPbEUE96ROWU=";
            "hyper-util-0.1.2" = "sha256-x1H4keSXHtU7TRv+cUx+g3K0gX0vj/B/rwM0fGYVN74=";
            "rocket-0.6.0-dev" = "sha256-+wbjyKjZK7IlShIBU6TBqdhrGEL246RWZU9aoKYlFUo=";
            "rocket_csrf-0.3.0" = "sha256-MiQ2/a5z3bBrZzw7CpUBEPf9PZTWsSWMsNngNTpBEOQ=";
          };
        };

        buildInputs = [
          final.pkg-config
          final.openssl
        ] ++ final.lib.optionals final.stdenv.isDarwin [
          final.darwin.apple_sdk.frameworks.SystemConfiguration
        ];

        meta = with final.lib; {
          description = "BlueChips - finances for people with shared expenses";
          homepage = "https://github.com/quentinmit/bluechips/";
          license = licenses.gpl2Plus;
          maintainers = [maintainers.quentin];
        };
      };
    });
  in
    (flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ overlay ];
        };
      in {
        packages = rec {
          inherit (pkgs) bluechips-rs;
          default = bluechips-rs;
        };
      })) // {
        overlays.default = overlay;
      };
}
