{
  description = "Boluo Server";
  inputs = {
    nixpkgs.url = github:NixOS/nixpkgs/nixos-20.03;
    rust-overlay.url = "github:oxalica/rust-overlay";
    import-cargo.url = github:edolstra/import-cargo;
  };

  outputs = { self, nixpkgs, rust-overlay, import-cargo }:
  let
    system = "x86_64-linux";
    inherit (import-cargo.builders) importCargo;
    overlays = [ (import rust-overlay) ];
    pkgs = import nixpkgs {
      inherit system overlays;
    };
  in {

    defaultPackage.x86_64-linux =
      with import nixpkgs { system = system; };
      stdenv.mkDerivation {
        name = "boluo-server";
        src = self;

        nativeBuildInputs = [
          # setupHook which makes sure that a CARGO_HOME with vendored dependencies
          # exists
          (importCargo { lockFile = ./Cargo.lock; inherit pkgs; }).cargoHome
          pkgs.openssl.dev
          pkgs.pkg-config
          # Build-time dependencies
          pkgs.rust-bin.nightly.latest.default
          pkgs.rust-bin.nightly.latest.cargo
        ];

        buildPhase = ''
          cargo build --release --offline
        '';

        installPhase = ''
          install -Dm775 ./target/release/server $out/bin/server
        '';

      };

  };
}
