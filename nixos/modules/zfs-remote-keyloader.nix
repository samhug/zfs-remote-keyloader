# Remote ZFS Unlock module

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.boot.initrd.zfs-remote-keyloader;
in
{

  options.boot.initrd.zfs-remote-keyloader = {

    enable = mkOption {
      type = types.bool;
      default = false;
      description = ''
        Start the zfs-remote-keyloader service during initrd boot.
        Requires boot.initrd.networking.enable = true
      '';
    };

    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ../../package.nix { };
      description = ''
        zfs-remote-keyloader package
      '';
    };

    zfsDataset = mkOption {
      type = types.str;
      description = ''
        The zfs dataset to load keys for.
      '';
    };

    listenAddr = mkOption {
      type = types.str;
      default = "0.0.0.0:80";
      description = ''
        Address and port the HTTP service should listen on.
      '';
    };

    postLoadCommands = mkOption {
      type = types.lines;
      default = "";
      description = ''
        Commands to be executed after zfs-remote-keyloader exits
      '';
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = config.boot.initrd.systemd.enable;
        message = "boot.initrd.systemd.enable required for zfs-remote-keyloader";
      }
      {
        assertion = config.boot.initrd.systemd.network.enable;
        message = "boot.initrd.systemd.network.enable required for zfs-remote-keyloader";
      }
    ];

    boot.initrd.systemd.initrdBin = [ cfg.package ];

    boot.initrd.systemd.services.zfs-remote-keyloader = {
      wantedBy = [ "initrd.target" ];
      after = [
        "network.target"
        "local-fs.target"
      ];
      unitConfig.DefaultDependencies = "no";
      script = ''
        set -e
        ${cfg.package}/bin/zfs-remote-keyloader --listen-addr '${cfg.listenAddr}' --zfs-dataset '${cfg.zfsDataset}'
        ${cfg.postLoadCommands}
      '';
      environment.RUST_LOG = "info";
      serviceConfig.Restart = "on-failure";
      serviceConfig.RestartSec = 3;
      serviceConfig.StateDirectory = "zfs-remote-keyloader";
    };

  };

}

