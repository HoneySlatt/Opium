{
  description = "Music player for Apple Music, SoundCloud and Navidrome";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ];
      # Loaded at run time by winit and wgpu.
      runtimeLibs =
        pkgs: with pkgs; [
          wayland
          libxkbcommon
          vulkan-loader
          libGL
        ];
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "music";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            cargoBuildFlags = [ "-p" "app-desktop" ];
            # The tests are run from the dev shell; they need no system here.
            doCheck = false;
            nativeBuildInputs = with pkgs; [
              pkg-config
              rustPlatform.bindgenHook
              makeWrapper
            ];
            buildInputs = with pkgs; [
              ffmpeg
              pipewire
            ];
            postInstall = ''
              install -Dm644 packaging/music.desktop $out/share/applications/music.desktop
              install -Dm644 packaging/music.svg $out/share/icons/hicolor/scalable/apps/music.svg
              wrapProgram $out/bin/music \
                --prefix LD_LIBRARY_PATH : ${
                  pkgs.lib.makeLibraryPath (
                    runtimeLibs pkgs
                    ++ (with pkgs.xorg; [
                      libX11
                      libXcursor
                      libXi
                    ])
                  )
                }
            '';
            meta = {
              description = "Apple Music, SoundCloud and Navidrome in one desktop player";
              license = pkgs.lib.licenses.gpl3Plus;
              mainProgram = "music";
              platforms = pkgs.lib.platforms.linux;
            };
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
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
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (runtimeLibs pkgs);
          };
        }
      );
    };
}
