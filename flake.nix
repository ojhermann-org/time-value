{
  description = "Type-safe time-value-of-money calculations in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      git-hooks,
      ...
    }:
    let
      systems = [
        "aarch64-darwin"
        "x86_64-linux"
        "aarch64-linux"
      ];

      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ (import rust-overlay) ];
            }
          )
        );

      # git-hooks.nix: fast pre-commit hooks, kept as a local convenience. The
      # heavy checks (clippy/test/deny) run through the same flake in CI as
      # `nix develop -c cargo …` — see docs/adr/0012-ci-and-release-automation.md.
      mkPreCommit =
        pkgs:
        git-hooks.lib.${pkgs.stdenv.hostPlatform.system}.run {
          src = ./.;
          hooks = {
            rustfmt.enable = true;
            nixfmt.enable = true;
            typos.enable = true;
            trim-trailing-whitespace.enable = true;
            end-of-file-fixer.enable = true;
            check-toml.enable = true;
            check-merge-conflicts.enable = true;
            detect-private-keys.enable = true;
          };
        };
    in
    {
      # `nix flake check` validates the pre-commit hook set. The workspace's own
      # verification (fmt/clippy/test/deny) is run via `nix develop -c cargo …`
      # (locally, and in CI) so there is one definition of each tool.
      checks = forAllSystems (pkgs: {
        pre-commit = mkPreCommit pkgs;
      });

      devShells = forAllSystems (
        pkgs:
        let
          # The toolchain (with clippy/rustfmt/rust-src) is pinned by
          # rust-toolchain.toml; oxalica/rust-overlay reads it.
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          pre-commit = mkPreCommit pkgs;
        in
        {
          default = pkgs.mkShell {
            packages = [
              rustToolchain
              pkgs.bacon
              pkgs.cargo-nextest
              pkgs.cargo-deny
              pkgs.nixfmt
            ];
            buildInputs =
              pre-commit.enabledPackages ++ nixpkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
            # Installs the git-hooks managed pre-commit hook on shell entry.
            inherit (pre-commit) shellHook;
          };

          # A minimal 1.85 toolchain to verify the core library's MSRV
          # (docs/adr/0017-per-crate-msrv-core-1.85.md). The workspace builds on
          # 1.88 (rust-toolchain.toml); only `cargo test -p time_value` runs here.
          msrv = pkgs.mkShell {
            packages = [
              pkgs.rust-bin.stable."1.85.0".minimal
            ]
            ++ nixpkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
          };
        }
      );

      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
