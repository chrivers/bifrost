![](doc/logo-title-640x160.png)

# Bifrost Bridge

Bifrost enables you to emulate a Philips Hue Bridge to control lights, groups
and scenes from [Zigbee2Mqtt](https://www.zigbee2mqtt.io/).

If you are already familiar with [DiyHue](https://github.com/diyhue/diyHue), you
might like to read the [comparison with DiyHue](doc/comparison-with-diyhue.md).

Questions, feedback, comments? Join us on discord

[![Join Valhalla on Discord](https://discordapp.com/api/guilds/1276604041727578144/widget.png?style=banner2)](https://discord.gg/YvBKjHBJpA)

## Installation guide

There are currently three ways you can install Bifrost:

1.  [Install manually](#manual) from source (recommended)
2.  [Install it via Docker](#docker) for container-based deployment.
3.  Install as Home Assistant Add-on. Please see the
    [bifrost-hassio](https://github.com/chrivers/bifrost-hassio) project for
    more information.

### Manual

To install Bifrost from source, you will need the following:

1.  The rust language toolchain (https://rustup.rs/)
2.  At least one zigbee2mqtt server to connect to
3.  The MAC address of the network interface you want to run the server on
4.  `build-essential` package for compiling the source code (on Debian/Ubuntu systems)

First, install a few necessary build dependencies:

```sh
sudo apt install build-essential pkg-config libssl3 libssl-dev
```

When you have these things available, install bifrost:

```sh
cargo install --git https://github.com/chrivers/bifrost.git
```

After Cargo has finished downloading, compiling, and installing Bifrost, you
should have the "bifrost" command available to you.

The last step is to create a configuration for bifrost, `config.yaml`.

Here's a minimal example:

```yaml
bridge:
  name: Bifrost
  mac: 00:11:22:33:44:55
  ipaddress: 10.12.0.20
  netmask: 255.255.255.0
  gateway: 10.12.0.1
  timezone: Europe/Copenhagen

z2m:
  server1:
    url: ws://10.0.0.100:8080
```

Please adjust this as needed. Particularly, make **sure** the "mac:" field
matches a mac address on the network interface you want to serve requests from.

Make sure to read the [configuration reference](doc/config-reference.md) to
learn how to adjust the configuration file.

This mac address if used to generate a self-signed certificate, so the Hue App
will recognize this as a "real" Hue Bridge. If the mac address is incorrect,
this will not work. [How to find your mac address](doc/how-to-find-mac-linux.md).

Now you can start Bifrost. Simple start the "bifrost" command from the same
directory where you put the `config.yaml`:

```sh
bifrost
```

At this point, the server should start: (log timestamps omitted for clarity)

```
  ===================================================================
   ███████████   ███     ██████                              █████
  ░░███░░░░░███ ░░░     ███░░███                            ░░███
   ░███    ░███ ████   ░███ ░░░  ████████   ██████   █████  ███████
   ░██████████ ░░███  ███████   ░░███░░███ ███░░███ ███░░  ░░░███░
   ░███░░░░░███ ░███ ░░░███░     ░███ ░░░ ░███ ░███░░█████   ░███
   ░███    ░███ ░███   ░███      ░███     ░███ ░███ ░░░░███  ░███ ███
   ███████████  █████  █████     █████    ░░██████  ██████   ░░█████
  ░░░░░░░░░░░  ░░░░░  ░░░░░     ░░░░░      ░░░░░░  ░░░░░░     ░░░░░
  ===================================================================

  DEBUG bifrost > Configuration loaded successfully
  DEBUG bifrost::server::certificate > Found existing certificate for bridge id [001122fffe334455]
  DEBUG bifrost::state               > Existing state file found, loading..
  INFO  bifrost::mdns                > Registered service bifrost-001122334455._hue._tcp.local.
  INFO  bifrost                      > Serving mac [00:11:22:33:44:55]
  DEBUG bifrost::state               > Loading certificate from [cert.pem]
  INFO  bifrost::server              > http listening on 10.12.0.20:80
  INFO  bifrost::server              > https listening on 10.12.0.20:443
  INFO  bifrost::z2m                 > [server1] Connecting to ws://10.0.0.100:8080
  DEBUG tungstenite::handshake::client > Client handshake done.
  DEBUG tungstenite::handshake::client > Client handshake done.
  DEBUG bifrost::z2m                   > [server1] Ignoring unsupported device Coordinator
  INFO  bifrost::z2m                   > [server1] Adding light IeeeAddress(000000fffe111111): [office_1] (TRADFRI bulb GU10 CWS 345lm)
  INFO  bifrost::z2m                   > [server1] Adding light IeeeAddress(222222fffe333333): [office_2] (TRADFRI bulb GU10 CWS 345lm)
  INFO  bifrost::z2m                   > [server1] Adding light IeeeAddress(444444fffe555555): [office_3] (TRADFRI bulb GU10 CWS 345lm)
...
```

The log output shows Bifrost talking with zigbee2mqtt, and finding some lights to control (office\_{1,2,3}).

At this point, you're running a Bifrost bridge.

The Philips Hue app should be able to find it on your network!

### Docker

#### Docker Installation

To install Bifrost with Docker, you will need the following:

1.  At least one zigbee2mqtt server to connect to
2.  The MAC address of the network interface you want to run the server on
3.  A running [Docker](https://docs.docker.com/engine/install/) instance
    with [Docker-Compose](https://docs.docker.com/compose/install/) installed
4.  Have `git` installed to clone this repository

Please choose one of the following installation methods:

- [Install using Docker Compose](doc/docker-compose-install.md) (recommended for most users)
- [Install using Docker Image](doc/docker-image-install.md) (for direct image pulls)

# Configuration

See [configuration reference](doc/config-reference.md).

# Problems? Questions? Feedback?

Please note: Bifrost is a very young project. Some things are incomplete, and/or
broken when they shouldn't be.

Consider joining us on discord:

[![Join Valhalla on Discord](https://discordapp.com/api/guilds/1276604041727578144/widget.png?style=banner2)](https://discord.gg/YvBKjHBJpA)

If you have any problems, questions or suggestions, feel free to [create an
issue](https://github.com/chrivers/bifrost/issues) on this project.

Also, pull requests are always welcome!
