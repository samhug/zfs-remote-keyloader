{ pkgs
, makeTest
, zfs-remote-keyloader
}:

makeTest {
  name = "zfs-remote-keyloader";

  nodes = {
    node = { pkgs, ... }: {

      virtualisation.emptyDiskImages = [ 128 ];

      boot.supportedFilesystems = [ "zfs" ];

      networking.hostId = "00000000";

      environment.systemPackages = with pkgs; [
        gptfdisk
        parted
        curl
        zfs
        zfs-remote-keyloader
      ];
    };
  };

  testScript =
    let
      zfsEncryptionKey = "password";
      zfsPool = "rpool";
      listenPort = toString 3000;
    in
    ''
      node.start()
      node.wait_for_unit("multi-user.target")
      node.succeed("modprobe zfs")
      node.succeed(
          "udevadm settle",
          "sgdisk --zap-all /dev/vdb",
          "partprobe",
          "sleep 3",
          "udevadm settle",
          "udevadm settle",
          "echo ${zfsEncryptionKey} | zpool create"
          + " -O mountpoint=none"
          + " -O encryption=aes-256-gcm"
          + " -O keyformat=passphrase"
          + " ${zfsPool} vdb",
          "zfs unload-key ${zfsPool}",
      )
      node.succeed("zfs get -H -o value keystatus ${zfsPool} | grep unavailable")
      node.succeed("zfs-remote-keyloader --help")

      # Run the service
      node.succeed(
        "systemd-run --unit=zfs-remote-keyloader.service -E PATH --"
        + " zfs-remote-keyloader"
        + " --listen-addr 127.0.0.1:${listenPort}"
        + " --zfs-dataset ${zfsPool}"
      )

      node.wait_for_open_port(${listenPort})
      node.succeed("curl http://127.0.0.1:${listenPort}/ >&2")
      node.succeed("curl -X POST -d 'key=${zfsEncryptionKey}' http://127.0.0.1:${listenPort}/loadkey >&2")

      # Make sure the key was loaded successfully
      node.succeed("zfs get -H -o value keystatus ${zfsPool} | grep available")

      # Wait for the service to shut down
      node.wait_until_fails("ps -ef | grep '[z]fs-remote-keyloader'")

      # If the process exits with code 0, then the service unit with be unloaded, so the following should fail
      node.fail("systemctl status zfs-remote-keyloader.service")
    '';
}
{
  inherit pkgs;
  inherit (pkgs) system;
}
