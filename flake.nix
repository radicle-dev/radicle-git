{
  description = "Improved types and functionality for using Git in Radicle";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-24.11";

    crane = {
      url = "github:ipetkov/crane";
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
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pname = "radicle-git";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };

      inherit (pkgs) lib;

      rustToolChain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
      craneLib = (crane.mkLib pkgs).overrideToolchain rustToolChain;

      srcFilters = path: type:
      # Allow data/git-platinum.tgz
        (lib.hasSuffix "\.tgz" path)
        ||
        # Default filter from crane (allow .rs files)
        (craneLib.filterCargoSources path type);

      src = lib.cleanSourceWith {
        src = ./.;
        filter = srcFilters;
      };

      # Common arguments can be set here to avoid repeating them later
      commonArgs = {
        inherit pname;
        inherit src;
        strictDeps = true;

        buildInputs =
          [
            pkgs.git
            # Add additional build inputs here
          ]
          ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];
      };

      # Build *just* the cargo dependencies, so we can reuse
      # all of that work (e.g. via cachix) when running in CI
      cargoArtifacts =
        craneLib.buildDepsOnly commonArgs;

      # Build the actual crate itself, reusing the dependency
      # artifacts from above.

      radicle-git-ext = craneLib.buildPackage (commonArgs
        // {
          inherit (craneLib.crateNameFromCargoToml {cargoToml = ./radicle-git-ext/Cargo.toml;});
          doCheck = false;
          inherit cargoArtifacts;
        });
      radicle-std-ext = craneLib.buildPackage (commonArgs
        // {
          inherit (craneLib.crateNameFromCargoToml {cargoToml = ./radicle-std-ext/Cargo.toml;});
          doCheck = false;
          inherit cargoArtifacts;
        });
      radicle-surf = craneLib.buildPackage (commonArgs
        // {
          inherit (craneLib.crateNameFromCargoToml {cargoToml = ./radicle-surf/Cargo.toml;});
          inherit cargoArtifacts;
          doCheck = false;
        });

      # Test crates
      radicle-git-ext-test = craneLib.buildPackage (commonArgs
        // {
          inherit (craneLib.crateNameFromCargoToml {cargoToml = ./radicle-git-ext/t/Cargo.toml;});
          inherit cargoArtifacts;
          doCheck = false;
        });
      radicle-surf-test = craneLib.buildPackage (commonArgs
        // {
          inherit (craneLib.crateNameFromCargoToml {cargoToml = ./radicle-surf/t/Cargo.toml;});
          inherit cargoArtifacts;
          doCheck = false;
        });
    in {
      # Formatter
      formatter = pkgs.alejandra;

      # Set of checks that are run: `nix flake check`
      checks = {
        # Build the crate as part of `nix flake check` for convenience
        inherit radicle-git-ext;
        inherit radicle-surf;
        inherit radicle-git-ext-test;
        inherit radicle-surf-test;

        # Run clippy (and deny all warnings) on the crate source,
        # again, reusing the dependency artifacts from above.
        #
        # Note that this is done as a separate derivation so that
        # we can block the CI if there are issues here, but not
        # prevent downstream consumers from building our crate by itself.
        clippy = craneLib.cargoClippy (commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

        doc = craneLib.cargoDoc (commonArgs
          // {
            inherit cargoArtifacts;
          });

        # Check formatting
        fmt = craneLib.cargoFmt {
          inherit pname;
          inherit src;
        };

        # Audit dependencies
        audit = craneLib.cargoAudit {
          inherit src advisory-db;
        };

        # Audit licenses
        deny = craneLib.cargoDeny {
          inherit pname;
          inherit src;
        };

        # Run tests with cargo-nextest
        nextest = craneLib.cargoNextest (commonArgs
          // {
            inherit cargoArtifacts;
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

      packages = {
        inherit radicle-git-ext;
        inherit radicle-surf;
        inherit radicle-git-ext-test;
        inherit radicle-surf-test;
      };

      devShells.default = craneLib.devShell {
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
