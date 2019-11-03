{ sources ? import ./sources.nix }:

let
  nixpkgs-mozilla = import "${sources.nixpkgs-mozilla}/rust-overlay.nix";
  local-overlay = self: super:
    {
      niv = import sources.niv {};

      rustPlatformCustom =
        let
          rust-channel = self.rustChannelOf
            { date = "2019-11-01"; channel = "nightly"; };

          rustc = rust-channel.rust.override {
            targets = [ "x86_64-unknown-linux-musl" ];
          };

          cargo = rust-channel.cargo;
        in
          self.makeRustPlatform { inherit rustc cargo; };
    };

in

import sources.nixpkgs
  {
    overlays =
      [
        nixpkgs-mozilla
        local-overlay
      ];
    config = {};
  }
