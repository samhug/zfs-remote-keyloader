{
  description = "zfs-remote-keyloader";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = inputs@{ self, nixpkgs, flake-utils, ... }: rec {
    nixosModules.zfs-remote-keyloader = import ./nixos/modules/zfs-remote-keyloader.nix;
    nixosModules.default = nixosModules.zfs-remote-keyloader;
  }
  //
  flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
    in
    rec {

      packages.zfs-remote-keyloader = pkgs.callPackage ./package.nix { };

      packages.default = packages.zfs-remote-keyloader;

      legacyPackages = packages;

      devShells.default = pkgs.mkShell {
        CARGO_INSTALL_ROOT = "${toString ./.}/.cargo";

        buildInputs = with pkgs; [ cargo rustc git ];
      };

      checks =
        let
          makeTest = import (inputs.nixpkgs + "/nixos/tests/make-test-python.nix");
        in
        {

          basic-test = import ./nixos/tests/basic-test.nix {
            inherit makeTest;
            inherit pkgs;
            inherit (packages) zfs-remote-keyloader;
          };

        };
    });
}
