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
        version = "1.0.3";
        pkgs = (import nixpkgs) { inherit system; };
        nativeBuildInputs = with pkgs; [ cmake pkg-config rustc cargo stdenv glib llvmPackages.libclang ];
        buildInputs = with pkgs; [ libadwaita librsvg ffmpeg makeWrapper ];
        mkPackage = { name }: pkgs.rustPlatform.buildRustPackage rec {
          cargoBuildFlags = [ "--package ${name}" ];
          cargoTestFlags = cargoBuildFlags;
          pname = name;
          inherit version;
          inherit buildInputs;
          inherit nativeBuildInputs;
          cargoLock.lockFile = ./Cargo.lock;
          src = gitignoreSource ./.;
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.glibc.dev}/include";
          postInstall = ''
            for dir in target/*/release/share; do
              cp -r $dir $out/share
            done
            if [ -f $out/bin/void-gui ]; then
              rm -fr $out/share/glib-2.0
              wrapProgram $out/bin/void-gui --set GSETTINGS_SCHEMA_DIR $out/share/gsettings-schemas/${pname}-${version}/glib-2.0/schemas
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
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo cargo-edit cmake pkg-config busybox fzf gcc ];
          buildInputs = buildInputs ++ [ pkgs.llvmPackages.libclang ];
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.glibc.dev}/include";
        };
      }
    );
}
