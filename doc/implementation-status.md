## Implementation status

### Legacy (V1 API)

| Feature     | Endpoint                             | Status       |
|-------------|--------------------------------------|--------------|
| Minimal API | `/api/config`, `/api/:userid/config` | ✅           |
| Lights      | `/api/:user/lights`                  | ✅ (partial) |
| Groups      | `/api/:user/groups`                  | ✅ (partial) |
| Scenes      | `/api/:user/scenes`                  | ✅ (partial) |
| Sensors     | `/api/:user/sensors`                 | ❌           |

| Endpoint                   | GET | PUT | POST | DELETE |
|----------------------------|-----|-----|------|--------|
| `/`                        | -   | -   | ✅   | -      |
| `/config`                  | ✅  | -   | -    | -      |
| `/:user`                   | ✅  | -   | -    | -      |
| `/:user/config`            | ✅  | ❌  | ❌   | ❌     |
| `/:user/lights`            | ✅  | ❌  | ❌   | ❌     |
| `/:user/groups`            | ✅  | ❌  | ❌   | ❌     |
| `/:user/scenes`            | ✅  | ❌  | ❌   | ❌     |
| `/:user/capabilities`      | ✅  | ❌  | ❌   | ❌     |
| `/:user/<other>`           | ❌  | ❌  | ❌   | ❌     |
| `/:user/lights/:id`        | ✅  | -   | -    | ❌     |
| `/:user/groups/:id`        | ✅  | -   | -    | ❌     |
| `/:user/scenes/:id`        | ✅  | -   | -    | ❌     |
| `/:user/lights/:id/state`  | -   | ✅  | -    | -      |
| `/:user/groups/:id/action` | -   | ✅  | -    | -      |


### Modern (V2 API)

| Feature         | Implemented | Notes                                                                                                    |
|-----------------|-------------|----------------------------------------------------------------------------------------------------------|
| Authentication  | ❌          | No authentication! Everybody has full access                                                             |
| Config          | ✅          |                                                                                                          |
| Event streaming | ✅          | Can send updates for lights, groups, rooms, scenes                                                       |
| Lights          | ✅          | Supports on/off, color temperature, full color                                                           |
| Groups          | ✅          | Automatically mapped to rooms                                                                            |
| Scenes          | ✅          | Scenes can be created, recalled, deleted. Scenes found in zigbee2mqtt will be imported, and auto-learned |

| Feature | GET | POST | PUT          | DELETE |
|---------|-----|------|--------------|--------|
| Lights  | ✅  | -    | ✅ (patial)  | -      |
| Groups  | ✅  | ❌   | ✅ (patial)  | ❌     |
| Scenes  | ✅  | ✅   | ✅ (partial) | ✅     |
