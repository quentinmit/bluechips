{
  description = "py-profinet";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }: let
    overlay = (final: prev: {
      bluechips-rs = final.callPackage ./bluechips-rs {};
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
