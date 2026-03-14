{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.framework-control;
in
{
  options.services.framework-control.enable = lib.mkEnableOption "Framework Control device hardware service";

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ pkgs.framework-tool ];

    systemd.services.framework-control = {
      description = "Framework Control Service";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      path = [ pkgs.framework-tool ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${pkgs.callPackage ./package.nix { }}/bin/framework-control";
        Restart = "on-failure";
        RestartSec = "5s";
        User = "root";
        Group = "root";
        StateDirectory = "framework-control";
      };

      environment = {
        FRAMEWORK_CONTROL_CONFIG = "/var/lib/framework-control/config.json";
      };
    };
  };
}
