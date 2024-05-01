{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, flake-utils, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        version = "1.0.0";
        pkgs = (import nixpkgs) { inherit system; };
        nativeBuildInputs = with pkgs; [ cmake pkg-config rustc cargo stdenv ];
        buildInputs = with pkgs; [ libadwaita librsvg ];
        mkPackage = { name }: pkgs.rustPlatform.buildRustPackage rec {
          cargoBuildFlags = [ "--package ${name}" ];
          cargoTestFlags = cargoBuildFlags;
          pname = name;
          inherit version;
          inherit buildInputs;
          inherit nativeBuildInputs;
          cargoLock.lockFile = ./Cargo.lock;
          src = ./.;
          postInstall = "
            if [ -d target/*/release/share ]; then
              cp -r target/*/release/share $out/share
            fi
          ";
        };
      in
      rec {
        formatter = nixpkgs.legacyPackages.${system}.nixpkgs-fmt;
        packages.void-lib = mkPackage { name = "void"; };
        packages.void-cli = mkPackage { name = "void-cli"; };
        packages.void-gui = mkPackage { name = "void-gui"; };
        packages.default = packages.void-cli;
        apps = rec {
          void-cli = { type = "app"; program = "${packages.default}/bin/void-cli"; };
          default = void-cli;
        };
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo cmake pkg-config busybox ];
          inherit buildInputs;
        };
      }
    );
}
