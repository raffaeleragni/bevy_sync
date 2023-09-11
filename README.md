# bevy_sync

![Sync](docs/sync.gif)

Plugin for synchronizing entities and components between server and its clients. This plugin is meant to support collaborative editing environment through editors and supports bi-directional updates between server and client.

Uses `bevy_renet`.

Current state is in development.

- [X] Entity synchronization
- [X] Component synchronization
- [X] Parent/Child entity synchronization
- [X] Asset synchronization
  - [x] Mesh
  - [X] SimpleMaterial

## Examples

Run both examples so the they connect to each other:

- `cargo run --example host`
- `cargo run --example client`

Then open the editor and change a component value in one to see it reflected in the other.
