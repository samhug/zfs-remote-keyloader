{ pkgs ? import ./nix/pkgs.nix {} }:

with
{
  inherit (pkgs.rustPlatformCustom.rust) rustc cargo;
};

pkgs.mkShell rec {
  name = "zfs-remote-keyloader";

  buildInputs = [
    cargo
    rustc
  ];

  shellHook = ''
    export name="${name}"
  '';
}
