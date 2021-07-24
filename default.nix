with import <nixpkgs> { };
let mkRustPlatform = { date, channel }: 
  let mozillaOverlay = fetchFromGitHub {
        owner = "mozilla";
        repo = "nixpkgs-mozilla";
        rev = "3f3fba4e2066f28a1ad7ac60e86a688a92eb5b5f";
        sha256 = "1mrj89gzrzhci4lssvzmmk31l715cddp7l39favnfs1qaijly814";
      };
      mozilla = callPackage "${mozillaOverlay.out}/package-set.nix" {};
      rustSpecific = (mozilla.rustChannelOf { inherit date channel; }).rust;
  in 
  makeRustPlatform {
    cargo = rustSpecific;
    rustc = rustSpecific;
  };
rustPlatform = mkRustPlatform {
      date = "2021-07-22";
      channel = "nightly";
    };
in
rustPlatform.buildRustPackage rec {
  pname = "boluo-server";
  version = "0.1.0";
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ openssl ];
  doCheck = false;

  src = ./.;

  cargoSha256 = "0zx4gd4232shg9cc8q4lqxfrxir86yx02kfqs7j1xxrzajpyygyw";
}
    