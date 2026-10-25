{
  description = "invoiceninjars: a Rust client for the Invoice Ninja v5 API";

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
      in
      {
        devShells.default = shellFor rustToolchain;

        # Verifies the declared MSRV still builds:
        #   nix develop .#msrv --command cargo test
        devShells.msrv = shellFor msrvToolchain;
      });
}
