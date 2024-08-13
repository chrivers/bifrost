![](doc/logo-title-640x160.png)

# Bifrost Bridge

Bifrost enables you to emulate a Philips Hue Bridge to control lights, groups and scenes from [Zigbee2Mqtt](https://www.zigbee2mqtt.io/).

## Comparison

| Feature                              | DiyHue                          | Bifrost                         |
|--------------------------------------|---------------------------------|---------------------------------|
| Use Hue Bridge as backend            | ✅                              | ❌                              |
| Usable from Homeassistant            | ✅ (as a Hue Bridge)            | ❌ (missing Hue V1 API)         |
| Control individual lights            | ✅                              | ✅                              |
| Good performance for groups of light | ❌                              | ✅                              |
| Connect to zigbee2mqtt               | (✅) (but only one server)      | ✅ (multiple servers supported) |
| Auto-detection of color features     | ❌ (needs manual configuration) | ✅                              |
| Create zigbee2mqtt scenes            | ❌                              | ✅                              |
| Recall zigbee2mqtt scenes            | ❌                              | ✅                              |
| Learn zigbee2mqtt scenes             | ❌                              | ✅                              |
| Delete zigbee2mqtt scenes            | ❌                              | ✅                              |
| Join new zigbee lights               | ✅                              | ❌                              |
| Live state of lights in Hue app      | ❌ [^1]                         | ✅                              |
| Multiple type of backends            | ✅                              | ❌ (only zigbee2mqtt)           |
| Entertainment zones                  | ✅                              | ❌ (planned)                    |
| Routines / Wake up / Go to sleep     | ✅                              | ❌ (planned)                    |
|                                      |                                |                                |

[^1]: Light state synchronization (i.e. consistency between hue emulator, hue
    app and reality) seems to be, unfortunately, somewhat brittle in DiyHue. See
    for example:

 * https://github.com/diyhue/diyHue/issues/883
 * https://github.com/diyhue/diyHue/issues/835
 * https://github.com/diyhue/diyHue/issues/795

## Implementation status

### Legacy (V1 API)

| Feature | Endpoint                             | Status |
|------------------|--------------------------------------|--------|
| Minimal API      | `/api/config`, `/api/:userid/config` | ✅     |
| Lights           | `/api/:user/lights`                  | ❌     |
| Groups           | `/api/:user/groups`                  | ❌     |
| Scenes           | `/api/:user/scenes`                  | ❌     |
| Groups           | `/api/:user/groups`                  | ❌     |
| Sensors          | `/api/:user/sensors`                 | ❌     |

### Modern (V2 API)

| Feature         | Implemented | Notes                                                                                                    |
|-----------------|-------------|----------------------------------------------------------------------------------------------------------|
| Authentication  | ❌          | No authentication! Everybody has full access                                                             |
| Config          | ✅          |                                                                                                          |
| Event streaming | ✅          | Can send updates for lights, groups, rooms, scenes                                                       |
| Lights          | ✅          | Supports on/off, color temperature, full color                                                           |
| Groups          | ✅          | Automatically mapped to rooms                                                                            |
| Scenes          | ✅          | Scenes can be created, recalled, deleted. Scenes found in zigbee2mqtt will be imported, and auto-learned |
