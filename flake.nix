{
  inputs = {
    flake-utils.url = github:numtide/flake-utils;
    naersk.url = github:nix-community/naersk;
    nixpkgs.url = github:NixOS/nixpkgs/nixpkgs-unstable;
  };
  outputs = { self, flake-utils, naersk, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        version = "1.0.0";
        pkgs = (import nixpkgs) { inherit system; };
        naersk' = pkgs.callPackage naersk { };
        buildInputs = with pkgs; [
          atkmm
          brotli
          bzip2
          cairo
          fontconfig
          gdk-pixbuf
          glib
          gtk3
          libglvnd
          libpng
          libuuid
          pango
        ];
        mkPackage = { name, buildInputs ? [ ] }: naersk'.buildPackage {
          cargoBuildOptions = opts: opts ++ [ "--package" name ];
          inherit buildInputs;
          inherit name;
          inherit version;
          nativeBuildInputs = with pkgs;[ cmake pkgconfig ];
          src = ./.;
        };
      in
      rec {
        formatter = nixpkgs.legacyPackages.${system}.nixpkgs-fmt;
        packages.void-lib = mkPackage { name = "void"; };
        packages.void-cli = mkPackage { name = "void-cli"; };
        packages.void-gui = mkPackage { name = "void-gui"; inherit buildInputs; };
        packages.default = packages.void-cli;
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo cmake pkgconfig ];
          inherit buildInputs;
        };
      }
    );
}
