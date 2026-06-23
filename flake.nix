{
  description = "Type-safe time-value-of-money calculations in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
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
      crane,
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

      # Everything a system needs to build/check the crate, derived once per system.
      mkToolset =
        pkgs:
        let
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
          src = craneLib.cleanCargoSource ./.;
          commonArgs = {
            inherit src;
            strictDeps = true;
            buildInputs = nixpkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        {
          inherit
            rustToolchain
            craneLib
            src
            commonArgs
            cargoArtifacts
            ;
        };
    in
    {
      checks = forAllSystems (
        pkgs:
        let
          t = mkToolset pkgs;
        in
        {
          build = t.craneLib.buildPackage (t.commonArgs // { inherit (t) cargoArtifacts; });

          clippy = t.craneLib.cargoClippy (
            t.commonArgs
            // {
              inherit (t) cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            }
          );

          test = t.craneLib.cargoTest (t.commonArgs // { inherit (t) cargoArtifacts; });

          doc = t.craneLib.cargoDoc (t.commonArgs // { inherit (t) cargoArtifacts; });

          fmt = t.craneLib.cargoFmt { inherit (t) src; };

          # git-hooks.nix: fast pre-commit hooks. Heavy clippy/test live in the
          # crane checks above and in `nix flake check` / CI.
          pre-commit = git-hooks.lib.${pkgs.stdenv.hostPlatform.system}.run {
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
        }
      );

      devShells = forAllSystems (
        pkgs:
        let
          t = mkToolset pkgs;
          pre-commit = self.checks.${pkgs.stdenv.hostPlatform.system}.pre-commit;
        in
        {
          default = pkgs.mkShell {
            packages = [
              t.rustToolchain
              pkgs.bacon
              pkgs.cargo-nextest
              pkgs.nixfmt
            ];
            # Installs the git-hooks managed pre-commit hook on shell entry.
            inherit (pre-commit) shellHook;
            buildInputs = pre-commit.enabledPackages;
          };
        }
      );

      packages = forAllSystems (
        pkgs:
        let
          t = mkToolset pkgs;
        in
        {
          default = t.craneLib.buildPackage (t.commonArgs // { inherit (t) cargoArtifacts; });
        }
      );

      formatter = forAllSystems (pkgs: pkgs.nixfmt);
    };
}
