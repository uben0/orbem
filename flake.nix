{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }:
    utils.lib.eachDefaultSystem(system:
      let pkgs = nixpkgs.legacyPackages.${system}; in
      {
        devShell = pkgs.mkShell {
          buildInputs = [
            pkgs.pkg-config
            pkgs.alsa-lib
            pkgs.vulkan-loader
            pkgs.libudev-zero
            pkgs.libxkbcommon
            pkgs.xorg.libX11
            pkgs.xorg.libXcursor
            pkgs.xorg.libXi
          ];
          shellHook = ''
            exec ${pkgs.fish}/bin/fish
          '';
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.vulkan-loader
            pkgs.libxkbcommon
            pkgs.xorg.libX11
            pkgs.xorg.libXcursor
            pkgs.xorg.libXi
          ];
          fish_color_user = "yellow";
          fish_color_cwd = "yellow";
        };
      }
    );
}
