{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    gitignore.url = "github:hercules-ci/gitignore.nix";
    gitignore.inputs.nixpkgs.follows = "nixpkgs";
  };
  outputs = { self, flake-utils, nixpkgs, gitignore }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        inherit (gitignore.lib) gitignoreSource;
        version = "1.0.0";
        pkgs = (import nixpkgs) { inherit system; };
        nativeBuildInputs = with pkgs; [ cmake pkg-config rustc cargo stdenv glib ];
        buildInputs = with pkgs; [ libadwaita librsvg makeWrapper ];
        mkPackage = { name }: pkgs.rustPlatform.buildRustPackage rec {
          cargoBuildFlags = [ "--package ${name}" ];
          cargoTestFlags = cargoBuildFlags;
          pname = name;
          inherit version;
          inherit buildInputs;
          inherit nativeBuildInputs;
          cargoLock.lockFile = ./Cargo.lock;
          src = gitignoreSource ./.;
          postInstall = ''
            for dir in target/*/release/share; do
              cp -r $dir $out/share
            done
            if [ -f $out/bin/void-gui ]; then
              wrapProgram $out/bin/void-gui --set GSETTINGS_SCHEMA_DIR $out/share/gsettings-schema/void-gui-${version}/glib-2.0/schemas
            fi
          '' ;
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
          nativeBuildInputs = with pkgs; [ rustc cargo cmake pkg-config busybox fzf ];
          inherit buildInputs;
        };
      }
    );
}
