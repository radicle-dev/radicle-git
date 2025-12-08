{
  description = "Improved types and functionality for using Git in Radicle";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-25.11";

    crane = {
      url = "github:ipetkov/crane";
    };

    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  nixConfig = {
    keepOutputs = true;
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    fenix,
    flake-utils,
    advisory-db,
    rust-overlay,
    ...
  } @ inputs:
    flake-utils.lib.eachDefaultSystem (system: let
      pname = "radicle-git";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };

      inherit (pkgs) lib;

      msrv = let
        msrv = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.rust-version;
      in rec {
        toolchain = pkgs.rust-bin.stable.${msrv}.default;
        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
        commonArgs = mkCommonArgs craneLib;
      };

      rustup = rec {
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
        commonArgs = mkCommonArgs craneLib;
      };

      srcFilters = path: type:
      # Allow data/git-platinum.tgz
        (lib.hasSuffix "\.tgz" path)
        ||
        # Default filter from crane (allow .rs files)
        (rustup.craneLib.filterCargoSources path type);

      src = lib.cleanSourceWith {
        src = ./.;
        filter = srcFilters;
      };

      basicArgs = {
        inherit src;
        pname = "radicle-git";
        version = "0.1.0";
        strictDeps = true;
      };

      # Common arguments can be set here to avoid repeating them later
      mkCommonArgs = craneLib:
        basicArgs
        // {
          # Build *just* the cargo dependencies, so we can reuse
          # all of that work (e.g. via cachix) when running in CI
          cargoArtifacts = craneLib.buildDepsOnly basicArgs;

          nativeBuildInputs = with pkgs; [
            git
          ];
          buildInputs = lib.optionals pkgs.stdenv.buildPlatform.isDarwin (with pkgs; [
            darwin.apple_sdk.frameworks.Security
            libiconv
          ]);
        };

      buildCrate = rust: name: let
        # Test crates live under `{crate}/t` but are called `{crate}-t`.
        # Check if it is a test crate, and safely remove the suffix and build the Cargo.toml path.
        isTest = lib.hasSuffix "-test" name;
        pname = lib.removeSuffix "-test" name;
        cargoToml = src + "/${pname}${lib.optionalString isTest "/t"}/Cargo.toml";
      in
        rust.craneLib.buildPackage (rust.commonArgs
          // {
            inherit (rust.craneLib.crateNameFromCargoToml {inherit cargoToml;}) pname version;
            cargoExtraArgs = "-p ${pname}";
            doCheck = false;
          });
      buildCrates = {
        rust ? rustup,
        prefix ? "",
      }:
        builtins.listToAttrs (map
          (name: lib.nameValuePair (prefix + name) (buildCrate rust name))
          [
            "radicle-std-ext"
            "radicle-git-ext"
            "radicle-git-ext-test"
            "radicle-surf"
            "radicle-surf-test"
          ]);
    in {
      # Formatter
      formatter = pkgs.alejandra;

      checks =
        (buildCrates {
          rust = msrv;
          prefix = "msrv-";
        })
        // {
          pre-commit-check = inputs.git-hooks.lib.${system}.run {
            src = ./.;
            settings.rust.check.cargoDeps = pkgs.rustPlatform.importCargoLock {lockFile = ./Cargo.lock;};
            default_stages = [
              "pre-commit"
              "pre-push"
            ];
            hooks = {
              alejandra.enable = true;
              codespell = {
                enable = true;
                entry = "${lib.getExe pkgs.codespell} -w";
                types = ["text"];
              };
              rustfmt = {
                enable = true;
                fail_fast = true;
                packageOverrides.rustfmt = rustup.toolchain;
              };
              cargo-check = {
                enable = true;
                name = "cargo check";
                after = ["rustfmt"];
                fail_fast = true;
              };
              cargo-doc = let
                # We wrap `cargo` in order to set an environment variable that
                # gives us a non-zero exit on warning.
                command =
                  pkgs.writeShellScript
                  "cargo"
                  "RUSTDOCFLAGS='--deny warnings' ${lib.getExe' rustup.toolchain "cargo"} $@";
              in {
                enable = true;
                name = "cargo doc";
                after = ["rustfmt"];
                fail_fast = true;
                entry = "${command} doc --workspace --all-features --no-deps";
                files = "\\.rs$";
                pass_filenames = false;
              };
              clippy = {
                enable = true;
                name = "cargo clippy";
                stages = ["pre-push"]; # Only pre-push, because it takes a while.
                settings = {
                  allFeatures = true;
                  denyWarnings = true;
                };
                packageOverrides = {
                  cargo = rustup.toolchain;
                  clippy = rustup.toolchain;
                };
              };
              shellcheck.enable = true;
            };
          };

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          doc = rustup.craneLib.cargoDoc rustup.commonArgs;
          deny = rustup.craneLib.cargoDeny rustup.commonArgs;
          fmt = rustup.craneLib.cargoFmt basicArgs;

          audit = rustup.craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Run tests with cargo-nextest
          nextest = rustup.craneLib.cargoNextest (rustup.commonArgs
            // {
              partitions = 1;
              partitionType = "count";
              nativeBuildInputs = [
                # git is required so the sandbox can access it.
                pkgs.git
              ];
              # Ensure dev is used since we rely on env variables being
              # set in tests.
              buildPhase = ''
                export CARGO_PROFILE=dev;
              '';
            });
        };

      packages = buildCrates {};

      devShells.default = rustup.craneLib.devShell {
        # Extra inputs can be added here; cargo and rustc are provided by default.
        packages = [
          pkgs.cargo-deny
          pkgs.cargo-msrv
          pkgs.cargo-nextest
          pkgs.cargo-semver-checks
          pkgs.cargo-watch
          pkgs.ripgrep
          pkgs.rust-analyzer
        ];
      };
    });
}
