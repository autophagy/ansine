ansíne
  noun: a view, sight, figure

Ansíne is a lightweight, simple, slightly-configurable dashboard intended for
a linux home server. It displays simple system metrics like average cpu idle,
memory usage and uptime, as well as configurable links to services running
on the home server. Only intended to be run in Linux environments, like NixOS.

.. image:: screen.png
    :align: center

Building
--------

To build::

  λ nix build

Configuration
-------------

Ansíne expects an environment variable named ``ANSINE_CONFIG_PATH`` to be present and pointing to a JSON configuration file. An example configuration:

.. code-block:: json

  {
    "port": 3000,
    "nixosCurrentSystem": true,
    "refreshInterval": 2,
    "services": {
      "Jellyfin": {
        "description": "Media Player and indexer",
        "route": "/jellyfin"
      },
      "Vaultwarden": {
        "description": "Bitwarden compatible credential storage",
        "route": "/vault"
      }
    }
  }
