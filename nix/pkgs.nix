{ sources ? import ./sources.nix }:

let
  nixpkgs-mozilla = import "${sources.nixpkgs-mozilla}/rust-overlay.nix";
  local-overlay = self: super:
    let
      rust-channel = self.rustChannelOf
        { date = "2020-02-24"; channel = "nightly"; };

      rustc = rust-channel.rust.override {
        targets = [ "x86_64-unknown-linux-musl" ];
      };

      cargo = rust-channel.cargo;
    in
      {
        niv = import sources.niv {};

        rustPlatformCustom =
          self.makeRustPlatform { inherit rustc cargo; };

        naersk = self.callPackage sources.naersk { inherit cargo rustc; };
      };
in

import sources.nixpkgs
  {
    overlays = [
      nixpkgs-mozilla
      local-overlay
    ];
    config = {};
  }
