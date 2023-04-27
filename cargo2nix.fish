#!/usr/bin/env fish

LD_LIBRARY_PATH= nix run github:acristoffers/cargo2nix

sed -i 's/\[ "emscripten" \]//g' Cargo.nix
sed -i 's/\[ "metal" \]/(if hostPlatform.isMacOS then [ "metal" ] else [])/g' Cargo.nix
sed -i 's/\[ "dx11" \]/(if hostPlatform.isWindows then [ "dx11" ] else [])/g' Cargo.nix
sed -i 's/\[ "dx12" \]/(if hostPlatform.isWindows then [ "dx12" ] else [])/g' Cargo.nix
sed -i 's/\[ "windows_rs" \]/(if hostPlatform.isWindows then [ "windows_rs" ] else [])/g' Cargo.nix
sed -i 's/\[ "dxc_shader_compiler" \]/(if hostPlatform.isWindows then [ "dxc_shader_compiler" ] else [])/g' Cargo.nix

nix fmt
