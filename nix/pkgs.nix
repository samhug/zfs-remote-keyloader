{ sources ? import ./sources.nix }:

let
  nixpkgs-mozilla = import "${sources.nixpkgs-mozilla}/rust-overlay.nix";
  local-overlay = self: super:
    let
      rust-channel = self.rustChannelOf
        {
          channel = "nightly";
          date = "2020-03-06";
          sha256 = "1i8a4arwzadbbk7p8pfpgc13dpd4zljlcbz1iz5fpmrrgpy66369";
        };

      rustc = rust-channel.rust.override {
        extensions = [
          "clippy-preview"
          "rls-preview"
          "rustfmt-preview"
          "rust-analysis"
          "rust-std"
          "rust-src"
        ];
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
