# bevy_sync

![crates.io](https://img.shields.io/crates/v/bevy_sync)
![build](https://github.com/raffaeleragni/bevy_sync/actions/workflows/build.yml/badge.svg)

![Sync](docs/sync.gif)

Plugin for synchronizing entities and components between server and its clients.
Uses `bevy_renet`.

## Basic features

- [X] Entity synchronization
- [X] Entyty sync is based on UUIDs
- [X] Component synchronization
- [X] Parent/Child entity synchronization
- [X] Host switch / promotion
- [X] SimpleMaterial (through sync channel)
- [X] Serve assets through http
- [X] Asset: Mesh
  - [X] programmatically created mesh
  - [X] mesh from gltf: static
  - [X] rigged
  - [X] with morphs
- [X] Asset: Textures
- [X] Asset: Audio
- [X] Compressed Assets

## Advanced features

- [ ] Throttleable sync (time window queuing)
- [ ] Skippable channel for Unordered+Unreliable
  - [ ] Transform

**Asset are synchronized only if they are added to bevy by uuid.**

## Examples

Run both examples so the they connect to each other:

- `cargo run --example host`
- `cargo run --example client`

Then open the editor and change a component value in one to see it reflected in the other.

## Versions

Base version of bevy_sync is inherited from bevy version.

| bevy | bevy_sync |
| ---- | --------- |
| 0.12 | 0.12.x    |
| 0.13 | 0.13.x    |
| 0.14 | 0.14.x    |
| ...  | ...       |

