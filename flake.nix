{
  description = "Desktop MCP - MCP server for the Linux desktop";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Use Rust 1.85+ for edition 2024
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
          ];
        };

        # System libraries needed for XDG Portals, PipeWire, and AT-SPI
        buildInputs = with pkgs; [
          # D-Bus and XDG Portal deps
          dbus
          glib

          # PipeWire (includes libpipewire and libspa)
          pipewire

          # AT-SPI2 (Assistive Technology Service Provider Interface)
          at-spi2-core

          # OpenSSL (for HTTP transport)
          openssl
          pkg-config
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          rustToolchain
          # For bindgen (used by pipewire-sys)
          llvmPackages.libclang
          llvmPackages.clang
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          # Environment variables for build
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          PKG_CONFIG_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.llvmPackages.libclang.version}/include -isystem ${pkgs.glibc.dev}/include";
          # For cc-rs (used by libspa-sys)
          CFLAGS = "-isystem ${pkgs.glibc.dev}/include";

          shellHook = ''
            echo "🖥️  Desktop MCP Development Environment"
            echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
            echo ""
            echo "System dependencies:"
            echo "  • D-Bus: ${pkgs.dbus.version}"
            echo "  • PipeWire: ${pkgs.pipewire.version}"
            echo "  • AT-SPI2: ${pkgs.at-spi2-core.version}"
            echo ""
            echo "Available commands:"
            echo "  cargo build          - Build the project"
            echo "  cargo run            - Run in stdio mode"
            echo "  cargo test           - Run tests"
            echo "  cargo clippy         - Lint the code"
            echo ""
            echo "Make sure xdg-desktop-portal is running:"
            echo "  systemctl --user status xdg-desktop-portal"
            echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
          '';
        };

        # Build the package
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "desktopmcp";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          # Environment variables needed by bindgen (used by pipewire-sys)
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.llvmPackages.libclang.lib}/lib/clang/${pkgs.llvmPackages.libclang.version}/include -isystem ${pkgs.glibc.dev}/include";

          meta = with pkgs.lib; {
            description = "MCP server for the Linux desktop";
            homepage = "https://github.com/varbhat/desktopmcp";
            mainProgram = "desktopmcp";
            license = licenses.mit;
            maintainers = [ ];
          };
        };
      }
    );
}
