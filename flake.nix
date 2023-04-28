{
  inputs = {
    nixpkgs.url = github:NixOS/nixpkgs/nixpkgs-unstable;
    cargo2nix.url = github:acristoffers/cargo2nix/unstable;
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
        mkOvrd = { name, ovrds }: (pkgs.rustBuilder.rustLib.makeOverride {
          name = name;
          overrideAttrs = drv: {
            propagatedNativeBuildInputs = drv.propagatedNativeBuildInputs or [ ] ++ ovrds ++ [ pkgs.pkgconfig ];
          };
        });
        rustPkgs = pkgs.rustBuilder.makePackageSet {
          rustToolchain = pkgs.rust-bin.stable.latest.default;
          packageFun = import ./Cargo.nix;
          packageOverrides = pkgs: pkgs.rustBuilder.overrides.all ++ [
            (mkOvrd {
              name = "void-gui";
              ovrds = with pkgs; [
                bzip2
                libpng
                brotli
                libglvnd
                libuuid
              ];
            })
            (mkOvrd { name = "pango-sys"; ovrds = with pkgs; [ pango ]; })
            (mkOvrd { name = "gtk-sys"; ovrds = with pkgs; [ gtk3 ]; })
            (mkOvrd { name = "gdk-sys"; ovrds = with pkgs; [ gtk3 ]; })
            (mkOvrd { name = "gdk-pixbuf-sys"; ovrds = with pkgs; [ gdk-pixbuf ]; })
            (mkOvrd { name = "atk-sys"; ovrds = with pkgs; [ atkmm ]; })
            (mkOvrd { name = "cairo-sys-rs"; ovrds = with pkgs; [ cairo ]; })
            (mkOvrd { name = "glib-sys"; ovrds = with pkgs; [ glib ]; })
            (mkOvrd { name = "servo-fontconfig-sys"; ovrds = with pkgs; [ fontconfig ]; })
            (mkOvrd { name = "cmake"; ovrds = with pkgs; [ cmake ]; })
            (mkOvrd { name = "graphene-sys"; ovrds = with pkgs; [ graphene ]; })
            (mkOvrd { name = "gtk4-sys"; ovrds = with pkgs; [ gtk4 ]; })
            (mkOvrd { name = "gdk4-sys"; ovrds = with pkgs; [ gtk4 ]; })
            (mkOvrd { name = "libadwaita-sys"; ovrds = with pkgs; [ libadwaita ]; })
          ];
        };
        workspaceShell = (rustPkgs.workspaceShell {
          packages = with pkgs; [
            cmake
            bzip2
            libpng
            brotli
            libglvnd
            libuuid
            pango
            gtk3
            gdk-pixbuf
            cairo
            glib
            fontconfig
            atkmm
          ];
        });
      in
      rec {
        formatter = pkgs.nixpkgs-fmt;
        devShells = { default = workspaceShell; };
        packages = {
          void = (rustPkgs.workspace.void { }).bin;
          void-cli = (rustPkgs.workspace.void-cli { }).bin;
          void-gui = (rustPkgs.workspace.void-gui { }).bin;
          default = packages.void-cli;
        };
        apps = rec {
          void-cli = { type = "app"; program = "${packages.void-cli}/bin/void-cli"; };
          void-gui = { type = "app"; program = "${packages.void-gui}/bin/void-gui"; };
          default = void-cli;
        };
      }
    );
}
