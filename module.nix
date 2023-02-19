{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.ansine;
  user = "ansine";
  group = user;
  cfgFile = pkgs.writeText "ansine-config.json" (builtins.toJSON {
    inherit (cfg) services port nixosCurrentSystem refreshInterval;
  });
in
{
  options = {
    services.ansine = {
      enable = mkEnableOption (lib.mdDoc "Ansíne, a lightweight home server dashboard.");

      port = mkOption {
        type = types.port;
        default = 3000;
        description = lib.mdDoc "Port number Ansíne will listen to.";
      };

      nixosCurrentSystem = mkOption {
        type = types.bool;
        default = true;
        description = lib.mdDoc "Whether to display the current NixOS generation via /run/current-system.";
      };

      refreshInterval = mkOption {
        type = types.int;
        default = 10;
        description = lib.mdDoc "The interval, in seconds, that the dashboard should refresh system metrics";
      };

      services = mkOption {
        default = { };
        description = lib.mdDoc "Services to expose on the Ansíne dashboard";
        example = {
          Jellyfin = {
            description = "Media system";
            route = "/jellyfin/";
          };
          Vaultwarden = {
            description = "Bitwarden compatible credential storage";
            route = "/vault/";
          };
        };
        type = types.attrsOf (types.submodule (_: {
          options = {
            description = mkOption {
              type = types.str;
              default = "";
              description = lib.mdDoc "Service description";
            };

            route = mkOption {
              type = types.str;
              default = "";
              description = lib.mdDoc "Service route from host";
            };
          };
        }));
      };
    };
  };

  config = mkIf cfg.enable {
    users.users.${user} = {
      inherit group;
      description = "Ansíne system user";
      isSystemUser = true;
    };

    users.groups = {
      ansine = { };
    };

    systemd.services = {
      ansine = {
        description = "Ansíne service";
        after = [ "network.target" ];
        environment = {
          ANSINE_CONFIG_PATH = cfgFile;
        };
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          Restart = "on-failure";
          User = user;
          Group = group;
          ExecStart = "${pkgs.ansine}/bin/ansine";
        };
      };
    };
  };
}
