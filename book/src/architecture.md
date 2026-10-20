# Architecture

## Crate Dependency Graph

```mermaid
graph BT
    core["tangibles-chess-core"]
    tangibles["tangibles-chess-tangibles"]
    engine["tangibles-chess-engine"]
    icons["tangibles-chess-icons"]
    app["tangibles-chess (app)"]
    daemon["tangible-objects daemon (external process)"]

    tangibles --> core
    engine --> core
    app --> core
    app --> tangibles
    app --> engine
    app --> icons
    tangibles -.->|"GraphQL (graphql-transport-ws + HTTP)"| daemon
```

| Crate | Role | Depends on |
| ----- | ---- | ---------- |
| `tangibles-chess-core` | Game domain: `GameSession` over shakmaty, `MoveRequest`, `UndoPlan`, `GamePhase`, config enums, autosave | shakmaty |
| `tangibles-chess-tangibles` | `TangibleSource` (GraphQL + mock), `BoardGeometry`, `FieldSetBuilder`, `MoveInference`, `OccupancyProbe`, `PlacementGate` | core |
| `tangibles-chess-engine` | UCI subprocess client (stockfish, ...) | core, shakmaty |
| `tangibles-chess-icons` | Custom icon set built from fonts-rs sources via `icons.toml` | fonts-rs-* |
| `tangibles-chess` | GTK4/libadwaita tabletop app | all |

## Data flow (tangible modes)

```mermaid
flowchart LR
    A["physical markers<br/>on touchscreen"] -->|"evdev"| B["tangible-objects daemon<br/>--filter --graphql-listen"]
    B -->|"uinput clone (suppressed fields)"| C["compositor / this app"]
    B -->|"fieldEvents subscription"| D["tangibles-chess-tangibles<br/>MoveInference / PlacementGate"]
    D -->|"MoveIntent / ZoneAction"| E["tangibles-chess-core<br/>GameSession"]
    E -->|"state"| F["tangibles-chess app<br/>TabletopSurface / LightGuide"]
```
