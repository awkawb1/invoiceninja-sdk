{
  description = "invoiceninja-sdk: a Rust client for the Invoice Ninja v5 API";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # Pinned rather than `stable.latest`, so the toolchain only moves
        # when this line does. `latest` drifts with the flake inputs, which
        # means a lint can start failing on an unrelated change.
        devVersion = "1.99.1";

        # Must match `rust-version` in Cargo.toml. The floor comes from the
        # dependency graph, not from this crate's own code: reqwest pulls in
        # url -> idna -> idna_adapter -> icu_*, and the icu crates require
        # 1.88. Check with:
        #   cargo metadata --format-version 1 \
        #     | jq -r '.packages[] | select(.rust_version) | .rust_version' \
        #     | sort -V | tail -1
        msrvVersion = "1.88.0";

        toolchainFor = version: extensions:
          pkgs.rust-bin.stable.${version}.default.override { inherit extensions; };

        rustToolchain = toolchainFor devVersion [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        msrvToolchain = toolchainFor msrvVersion [ ];

        shellFor = toolchain: pkgs.mkShell {
          # jq is for the MSRV recheck in CLAUDE.md, which pipes `cargo
          # metadata` through it; the rest is the toolchain plus what
          # openssl-sys needs to link.
          packages = [ toolchain pkgs.pkg-config pkgs.openssl pkgs.jq ];
          RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
        };

        # Built with the pinned dev toolchain, not nixpkgs' own rustc, so a
        # `nix build` matches what `nix develop` and CI compile with rather
        # than whatever stable nixpkgs happens to carry.
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in
      {
        devShells.default = shellFor rustToolchain;

        # Verifies the declared MSRV still builds:
        #   nix develop .#msrv --command cargo test
        devShells.msrv = shellFor msrvToolchain;

        # This is a library with no `[[bin]]` target (see the "No CLI, and
        # no binary at all" non-goal in GOALS.md), so the output directory
        # ends up empty: `cargoInstallHook` only copies binaries matching a
        # `[[bin]]` target out of `target/release`, and there are none to
        # copy. `nix build` is still meaningful as a build-and-test gate —
        # it compiles with the pinned toolchain and runs the full suite,
        # including doctests, inside the Nix sandbox (no network, matching
        # the "no credentials needed" property in CLAUDE.md) — just not as
        # a source of an installable artifact.
        packages.default = rustPlatform.buildRustPackage {
          pname = "invoiceninja-sdk";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];
          # The test suite needs no network (see CLAUDE.md), so the default
          # `cargo test` checkPhase runs unmodified in the sandboxed build.
        };
      });
}
