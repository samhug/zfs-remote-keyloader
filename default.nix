{ pkgs ? import ./nix/pkgs.nix {} }:

pkgs.rustPlatformCustom.buildRustPackage {
  name = "zfs-remote-keyloader";
  version = "0.0.1";

  src = ./.;

  cargoSha256 = "0pqvalddax45mhw856bg0l3b8qriabr4kv7wyny12qzcm1a6nrl3";
}
