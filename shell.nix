{ pkgs ? import ./nix/pkgs.nix {} }:

with
{
  inherit (pkgs.rustPlatformCustom.rust) rustc cargo;
};

pkgs.mkShell {
  name = "zfs-remote-keyloader";

  buildInputs = [
    rustc
    cargo
    #pkgs.zfs
  ];
}
