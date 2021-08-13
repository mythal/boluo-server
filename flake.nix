{
  description = "Boluo Server";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nmattia/naersk";
  };

  outputs = { self, nixpkgs, rust-overlay, naersk, utils }:
  utils.lib.eachDefaultSystem (system: let
    overlays = [ (import rust-overlay) ];
    pkgs = import nixpkgs {
      inherit system overlays;
    };
    rust-bin = pkgs.rust-bin.nightly."2021-07-30";
    naersk-lib = naersk.lib.${system}.override {
      cargo = rust-bin.default;
      rustc = rust-bin.default;
    };
  in rec {
    packages.boluo-server = naersk-lib.buildPackage {
      pname = "boluo-server";
      nativeBuildInputs = [
        rust-bin.default
        pkgs.openssl.dev
        pkgs.pkg-config
      ];
      root = ./.;
    };
    defaultPackage = packages.boluo-server;
    apps.server = utils.lib.mkApp {
      drv = packages.boluo-server;
    };
    devShell = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [ rust-bin.default rust-bin.cargo openssl.dev pkg-config ];
    };
  });
}
