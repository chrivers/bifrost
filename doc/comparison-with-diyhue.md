## Comparison with DiyHue

You might already be familiar with [DiyHue](https://github.com/diyhue/diyHue),
an existing project that aims to emulate a Philips Hue Bridge.

DiyHue is a well-established project, that integrates with countless
servers/services/light systems, and emulates many Hue Bridge features.

However, I have been frustrated with DiyHue's MQTT integration, and its fairly
poor performance when operating more than a handful of lights at a time. Since
DiyHue always sends individual messages to each light in a group, large rooms
can get quite slow (multiple seconds for every adjustment, no matter how minor).

As far as I know, DiyHue does not support Zigbee groups (or MQTT groups) at all,
whereas Bifrost is written specifically to present zigbee2mqtt groups as Hue
Bridge "rooms". For zigbee/mqtt use cases, this massively increases performance
and reliability.

Another thing about DiyHue that frustrates me to no end, is the lack of
(working) support for push notifications. If you use the Hue App to control a
DiyHue bridge, you will notice that it does not react to any changes from other
phones, home automation, etc. Also, the reported light states (on/off, color,
temperature, etc) are sometimes just wrong.

Overall, DieHue can do an impressive number of things, but it seems to have some
pretty rough edges.

Just to clarify, this is certainly not meant as an attack on DiyHue. I've
enjoyed using DiyHue,



| Feature                              | DiyHue                                  | Bifrost                                   |
|--------------------------------------|-----------------------------------------|-------------------------------------------|
| Language                             | Python                                  | Rust                                      |
| Project scope                        | Broad (supports countless integrations) | Narrow (specifically targets zigbee2mqtt) |
| Use Hue Bridge as backend            | ✅                                      | ❌                                        |
| Usable from Homeassistant            | ✅ (as a Hue Bridge)                    | ✅ (more testing needed)                  |
| Control individual lights            | ✅                                      | ✅                                        |
| Good performance for groups of light | ❌                                      | ✅                                        |
| Connect to zigbee2mqtt               | (✅) (but only one server)              | ✅ (multiple servers supported)           |
| Auto-detection of color features     | ❌ (needs manual configuration)         | ✅                                        |
| Create zigbee2mqtt scenes            | ❌                                      | ✅                                        |
| Recall zigbee2mqtt scenes            | ❌                                      | ✅                                        |
| Learn zigbee2mqtt scenes             | ❌                                      | ✅                                        |
| Delete zigbee2mqtt scenes            | ❌                                      | ✅                                        |
| Join new zigbee lights               | ✅                                      | ❌                                        |
| Live state of lights in Hue app      | ❌ [^1]                                 | ✅                                        |
| Multiple type of backends            | ✅                                      | ❌ (only zigbee2mqtt)                     |
| Entertainment zones                  | ✅                                      | ❌ (planned)                              |
| Routines / Wake up / Go to sleep     | ✅                                      | ❌ (planned)                              |

[^1]: Light state synchronization (i.e. consistency between hue emulator, hue
    app and reality) seems to be, unfortunately, somewhat brittle in DiyHue. See
    for example:

 * https://github.com/diyhue/diyHue/issues/883
 * https://github.com/diyhue/diyHue/issues/835
 * https://github.com/diyhue/diyHue/issues/795
