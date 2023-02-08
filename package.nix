{ rustPlatform, nix-gitignore }:

rustPlatform.buildRustPackage {
  pname = "zfs-remote-keyloader";
  version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
  src = nix-gitignore.gitignoreSource [ ] ./.;
  cargoLock.lockFile = ./Cargo.lock;
}