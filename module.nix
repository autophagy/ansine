{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.ansine;
  user = "ansine";
  group = user;
  settingsFormat = pkgs.formats.json { };
  configFile = settingsFormat.generate "config.json" cfg.settings;
in
{
  options = {
    services.ansine = {
      enable = mkEnableOption (lib.mdDoc "Ansíne, a lightweight home server dashboard.");

      settings = mkOption {
        inherit (settingsFormat) type;
        description = lib.mdDoc ''
          Ansíne configuration, see <https://github.com/autophagy/ansine#configuration>.
        '';
        example = {
          port = 3134;
          nixosCurrentSystem = true;
          refreshInterval = 3;
          services = {
            Jellyfin = {
              description = "Media system";
              route = "/jellyfin/";
            };
            Vaultwarden = {
              description = "Bitwarden compatible credential storage";
              route = "/vault/";
            };
          };
        };
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
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          Restart = "on-failure";
          User = user;
          Group = group;
          ExecStart = "${pkgs.ansine}/bin/ansine ${configFile}";
        };
      };
    };
  };
}
