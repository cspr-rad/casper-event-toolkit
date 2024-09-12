{
  description = "casper-event-toolkit";

  nixConfig = {
    extra-substituters = [
      "https://crane.cachix.org"
      "https://nix-community.cachix.org"
      "https://cspr.cachix.org"
    ];
    extra-trusted-public-keys = [
      "crane.cachix.org-1:8Scfpmn9w+hGdXH/Q9tTLiYAE/2dnJYRJP7kl80GuRk="
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "cspr.cachix.org-1:vEZlmbOsmTXkmEi4DSdqNVyq25VPNpmSm6qCs4IuTgE="
    ];
  };

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
    advisory-db.url = "github:rustsec/advisory-db";
    advisory-db.flake = false;
  };

  outputs = inputs@{ flake-parts, treefmt-nix, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      imports = [
        treefmt-nix.flakeModule
        # ./nixos
        # ./dummy-contract
      ];
      perSystem = { self', inputs', pkgs, lib, ... }:
        let
          rustToolchain = inputs'.fenix.packages.stable.toolchain;
          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

          packageAttrs = {
            pname = "casper-event-toolkit";

            src = lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.unions [
                ./Cargo.toml
                ./Cargo.lock
                ./src
                ./examples
              ];
            };

            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = with pkgs; [
              openssl.dev
            ] ++ lib.optionals stdenv.isDarwin [
              libiconv
              darwin.apple_sdk.frameworks.Security
              darwin.apple_sdk.frameworks.SystemConfiguration
            ];

            # the coverage report will run the tests
            doCheck = false;
          };
        in
        {
          devShells.default = pkgs.mkShell {
            inputsFrom = [ self'.packages.default ];
          };

          packages = {
            casper-event-toolkit-deps = craneLib.buildDepsOnly packageAttrs;

            casper-event-toolkit-docs = craneLib.cargoDoc (packageAttrs // {
              cargoArtifacts = self'.packages.casper-event-toolkit-deps;
            });

            casper-event-toolkit = craneLib.buildPackage (packageAttrs // {
              cargoArtifacts = self'.packages.casper-event-toolkit-deps;
            });

            default = self'.packages.casper-event-toolkit;
          };

          checks = {
            lint = craneLib.cargoClippy (packageAttrs // {
              cargoArtifacts = self'.packages.casper-event-toolkit-deps;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            });

            coverage-report = craneLib.cargoTarpaulin (packageAttrs // {
              cargoArtifacts = self'.packages.casper-vent-toolkit-deps;
            });
          };

          treefmt = {
            projectRootFile = ".git/config";
            programs.nixpkgs-fmt.enable = true;
            programs.rustfmt.enable = true;
            programs.rustfmt.package = craneLib.rustfmt;
            settings.formatter = { };
          };
        };
      flake = {
        herculesCI.ciSystems = [ "x86_64-linux" ];
      };
    };
}
