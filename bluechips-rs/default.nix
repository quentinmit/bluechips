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
      "hyper-util-0.1.2" = "sha256-x1H4keSXHtU7TRv+cUx+g3K0gX0vj/B/rwM0fGYVN74=";
      "rocket-0.6.0-dev" = "sha256-+wbjyKjZK7IlShIBU6TBqdhrGEL246RWZU9aoKYlFUo=";
      "rocket_csrf-0.3.0" = "sha256-MiQ2/a5z3bBrZzw7CpUBEPf9PZTWsSWMsNngNTpBEOQ=";
    };
  };

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
