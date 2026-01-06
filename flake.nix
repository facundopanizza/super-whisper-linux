{
  description = "SuperWhisper Linux - AI-powered voice to text for Linux";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Common build inputs for the project
        buildInputs = with pkgs; [
          # Audio (cpal/alsa)
          alsa-lib

          # OpenSSL for reqwest
          openssl

          # D-Bus for system tray (ksni)
          dbus

          # Wayland clipboard
          wl-clipboard
        ];

        # Native build inputs (build-time dependencies)
        nativeBuildInputs = with pkgs; [
          pkg-config
          cmake

          # For whisper.cpp compilation
          clang
        ];

      in {
        devShells.default = pkgs.mkShell {
          inherit buildInputs;

          nativeBuildInputs = nativeBuildInputs ++ [
            rustToolchain

            # Wayland tools for paste simulation
            pkgs.wtype

            # For Unix socket communication in trigger script
            pkgs.socat

            # Development tools
            pkgs.cargo-watch
            pkgs.cargo-edit
          ];

          # Environment variables for linking
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.alsa-lib.dev}/lib/pkgconfig:${pkgs.dbus.dev}/lib/pkgconfig";

          # For whisper-rs to find whisper.cpp
          WHISPER_DONT_GENERATE_BINDINGS = "1";

          # Clang for whisper.cpp
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

          shellHook = ''
            echo "SuperWhisper Linux development environment"
            echo "Rust: $(rustc --version)"
            echo ""
            echo "Available commands:"
            echo "  cargo build    - Build the project"
            echo "  cargo run      - Run the application"
            echo "  cargo watch    - Watch for changes and rebuild"
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "super-whisper-linux";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit buildInputs;

          nativeBuildInputs = nativeBuildInputs ++ [ pkgs.makeWrapper ];

          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

          # Wrap the binary to include runtime dependencies in PATH
          postInstall = ''
            wrapProgram $out/bin/super-whisper-linux \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.wtype pkgs.wl-clipboard pkgs.socat ]}
          '';

          meta = with pkgs.lib; {
            description = "AI-powered voice to text for Linux";
            homepage = "https://github.com/facundopanizza/super-whisper-linux";
            license = licenses.mit;
            platforms = platforms.linux;
            mainProgram = "super-whisper-linux";
          };
        };
      });
}
