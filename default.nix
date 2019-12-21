{ pkgs ? import ./nix/pkgs.nix {} }:

pkgs.naersk.buildPackage rec {
  name = "zfs-remote-keyloader-${version}";
  version = "0.1.0";

  src = pkgs.lib.cleanSource ./.;
}
