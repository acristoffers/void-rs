{
  inputs = {
    nixpkgs.url = github:NixOS/nixpkgs/nixpkgs-unstable;
    cargo2nix.url = github:cargo2nix/cargo2nix;
    flake-utils.url = github:numtide/flake-utils;
    rust-overlay.url = github:oxalica/rust-overlay;
  };
  outputs = inputs: with inputs;
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            cargo2nix.overlays.default
            rust-overlay.overlays.default
          ];
        };
        rustPkgs = pkgs.rustBuilder.makePackageSet {
          rustToolchain = pkgs.rust-bin.stable.latest.default;
          packageFun = import ./Cargo.nix;
          packageOverrides = pkgs: pkgs.rustBuilder.overrides.all ++ [
            (pkgs.rustBuilder.rustLib.makeOverride {
              name = "pango-sys";
              overrideAttrs = drv: {
                propagatedNativeBuildInputs = drv.propagatedNativeBuildInputs or [ ] ++ [
                  pkgs.pango
                ];
              };
            })
            (pkgs.rustBuilder.rustLib.makeOverride {
              name = "gtk-sys";
              overrideAttrs = drv: {
                propagatedNativeBuildInputs = drv.propagatedNativeBuildInputs or [ ] ++ [
                  pkgs.gtk3
                ];
              };
            })
            (pkgs.rustBuilder.rustLib.makeOverride {
              name = "gdk-sys";
              overrideAttrs = drv: {
                propagatedNativeBuildInputs = drv.propagatedNativeBuildInputs or [ ] ++ [
                  pkgs.gtk3
                ];
              };
            })
            (pkgs.rustBuilder.rustLib.makeOverride {
              name = "gdk-pixbuf-sys";
              overrideAttrs = drv: {
                propagatedNativeBuildInputs = drv.propagatedNativeBuildInputs or [ ] ++ [
                  pkgs.gdk-pixbuf
                ];
              };
            })
            (pkgs.rustBuilder.rustLib.makeOverride {
              name = "atk-sys";
              overrideAttrs = drv: {
                propagatedNativeBuildInputs = drv.propagatedNativeBuildInputs or [ ] ++ [
                  pkgs.atkmm
                ];
              };
            })
            (pkgs.rustBuilder.rustLib.makeOverride {
              name = "cairo-sys-rs";
              overrideAttrs = drv: {
                propagatedNativeBuildInputs = drv.propagatedNativeBuildInputs or [ ] ++ [
                  pkgs.cairo
                ];
              };
            })
            (pkgs.rustBuilder.rustLib.makeOverride {
              name = "glib-sys";
              overrideAttrs = drv: {
                propagatedNativeBuildInputs = drv.propagatedNativeBuildInputs or [ ] ++ [
                  pkgs.glib
                ];
              };
            })
            (pkgs.rustBuilder.rustLib.makeOverride {
              name = "servo-fontconfig-sys";
              overrideAttrs = drv: {
                propagatedNativeBuildInputs = drv.propagatedNativeBuildInputs or [ ] ++ [
                  pkgs.fontconfig
                ];
              };
            })
            (pkgs.rustBuilder.rustLib.makeOverride {
              name = "cmake";
              overrideAttrs = drv: {
                propagatedBuildInputs = drv.propagatedBuildInputs or [ ] ++ [
                  pkgs.cmake
                ];
              };
            })
          ];
        };
      in
      rec {
        formatter = pkgs.nixpkgs-fmt;
        packages = {
          void = (rustPkgs.workspace.void { }).bin;
          void-cli = (rustPkgs.workspace.void-cli { }).bin;
          void-gui = (rustPkgs.workspace.void-gui { }).bin;
          default = packages.void-cli;
        };
      }
    );
}
