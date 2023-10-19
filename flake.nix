{
  description = "Radicle Git";

  # Can use `input.` instead
  outputs = { self, nixpkgs, systems, flake-utils, rust-overlay, treefmt-nix, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # Small tool to iterate over each systems
        treefmtEval = treefmt-nix.lib.evalModule pkgs {
          # Used to find the project root
          projectRootFile = "flake.nix";
          programs.nixpkgs-fmt.enable = true;
          programs.rustfmt.package = pkgs.rust-bin.nightly."2022-07-01".rustfmt;
          programs.rustfmt.enable = true;
        };
      in
      {
        devShells.default = import ./shell.nix { inherit pkgs; };

        # formatter = pkgs.alejandra;
        # for `nix fmt`
        formatter = treefmtEval.config.build.wrapper;
        # for `nix flake check`
        checks = {
          # ci = pkgs.runCommand "ci/run" {
          #   src = ./.;
          #   _noChroot = true;
          #   nativeBuildInputs = self.devShells.${system}.default.nativeBuildInputs ++ [ pkgs.cargo pkgs.rust-bin.nightly."2022-07-01".rustfmt pkgs.clippy ];
          # } ''
          #   set -eou pipefail

          #   HOME=$(pwd)/rust-build
          #   mkdir $HOME
          #   export HOME

          #   mkdir $HOME/.cargo -p
          #   chmod +w -R $HOME/.cargo

          #   cp $src ./build -r
          #   cd ./build

          #   cargo fmt -- --check
          #   bash ./scripts/ci/lint
          #   bash ./scripts/ci/build
          #   bash ./scripts/ci/test
          #   bash ./scripts/ci/docs
          #   bash ./scripts/ci/advisory
          # '';
          formatting = treefmtEval.config.build.check self;
        };
      }
    );

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-23.05";
    rust-overlay.url = "https://github.com/oxalica/rust-overlay/archive/673e2d3d2a3951adc6f5e3351c9fce6ad130baed.tar.gz";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    flake-utils.url = "github:numtide/flake-utils";
  };
}
