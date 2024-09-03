{stdenv
, lib
, rustPlatform
, pkg-config
, openssl
, darwin
}: rustPlatform.buildRustPackage rec {
  pname = "bluechips-rs";
  version = "0.1.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "askama-0.12.1" = "sha256-tjkjf/2vSZQ2ljQakhsWFkGhJN/32JmRPbEUE96ROWU=";
      "rocket-0.6.0-dev" = "sha256-DweqUNz2JWMCDynqwO3CE9J6yi4lzdlzGhaNx/tlJ6M=";
      "rocket_csrf-0.3.0" = "sha256-MiQ2/a5z3bBrZzw7CpUBEPf9PZTWsSWMsNngNTpBEOQ=";
      "s2n-quic-h3-0.1.0" = "sha256-E5LSzRoN5aVc7CnPokYsny0QXQIzlu49zQ3INBmYX1E=";
    };
  };
  # Workaround for https://github.com/NixOS/nixpkgs/pull/300532
  cargoDepsHook = ''
    fixRocket() {
      echo cargoDepsCopy=$cargoDepsCopy
      sed -i '/workspace/d' $cargoDepsCopy/rocket*-0.6.0-dev/Cargo.toml
    }
    prePatchHooks+=(fixRocket)
  '';

  cargoBuildFlags = [
    "--workspace"
  ];

  nativeBuildInputs = [
    pkg-config
  ];
  buildInputs = [
    openssl
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  env.ROCKET_PUBLIC_PATH = "${../bluechips/public}";

  meta = with lib; {
    description = "BlueChips - finances for people with shared expenses";
    homepage = "https://github.com/quentinmit/bluechips/";
    license = licenses.gpl2Plus;
    maintainers = [maintainers.quentin];
  };
}
