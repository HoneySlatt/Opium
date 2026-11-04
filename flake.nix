{
  description = "Music player for Apple Music, SoundCloud and Navidrome";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Rust with the Android targets, for the Android app's core.
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
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
              # BoringSSL, for SoundCloud's changes sent as Chrome does: its
              # build patches the sources with git.
              cmake
              git
              perl
            ];
            buildInputs = with pkgs; [
              ffmpeg
              libopus
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
              description = "Apple Music, SoundCloud, Navidrome and your own files in one desktop player";
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
              # BoringSSL, for SoundCloud's changes sent as Chrome does.
              cmake
              perl
            ];
            # Animated cover art is decoded with FFmpeg's libraries; Opus
            # with libopus, which Symphonia has no decoder of. The window
            # drawn by GPUI links fontconfig, FreeType and xkbcommon too.
            buildInputs = with pkgs; [
              ffmpeg
              libopus
              fontconfig
              freetype
              libxkbcommon
              wayland
            ];
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (runtimeLibs pkgs);
          };
        }
        // nixpkgs.lib.optionalAttrs (system == "x86_64-linux") {
          # The Android app: its Rust core for the phone, and Gradle.
          android =
            let
              pkgs = import nixpkgs {
                inherit system;
                overlays = [ rust-overlay.overlays.default ];
                # Google's SDK, under its own licence: accepted for this
                # shell only, as the project's owner agreed.
                config = {
                  allowUnfree = true;
                  android_sdk.accept_license = true;
                };
              };
              buildTools = "36.1.0";
              ndk = "30.0.16248370";
              sdk =
                (pkgs.androidenv.composeAndroidPackages {
                  platformVersions = [ "37.0" ];
                  buildToolsVersions = [ buildTools ];
                  includeNDK = true;
                  ndkVersions = [ ndk ];
                }).androidsdk;
              home = "${sdk}/libexec/android-sdk";
              rust = pkgs.rust-bin.stable.latest.default.override {
                targets = [
                  "aarch64-linux-android"
                  "x86_64-linux-android"
                ];
              };
            in
            pkgs.mkShell {
              packages = [
                rust
                pkgs.cargo-ndk
                pkgs.jdk17
                pkgs.gradle_9
                sdk
              ];
              ANDROID_HOME = home;
              ANDROID_NDK_HOME = "${home}/ndk/${ndk}";
              JAVA_HOME = pkgs.jdk17.home;
              # Gradle's own aapt2 is built for other Linuxes; the SDK's runs here.
              GRADLE_OPTS = "-Dorg.gradle.project.android.aapt2FromMavenOverride=${home}/build-tools/${buildTools}/aapt2";
            };
        }
      );
    };
}
