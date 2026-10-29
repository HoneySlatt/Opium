{
  description = "Music player for Apple Music, SoundCloud and Navidrome";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ];
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          # Loaded at run time by winit and wgpu.
          runtimeLibs = with pkgs; [
            wayland
            libxkbcommon
            vulkan-loader
            libGL
          ];
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              rustc
              clippy
              rustfmt
              pkg-config
              pipewire
              rustPlatform.bindgenHook
            ];
            # Animated cover art is decoded with FFmpeg's libraries.
            buildInputs = with pkgs; [ ffmpeg ];
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
          };
        }
      );
    };
}
