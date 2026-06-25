# first-person

A first-person shooter prototype built with [Bevy](https://bevyengine.org/) `0.18`
and [`bevy_rapier3d`](https://github.com/dimforge/bevy_rapier) for physics. You walk
around procedural hilly terrain and shoot floating "balloon" targets against the
clock for points.

## Gameplay

- Shoot balloons before the 60-second round timer runs out.
- Chain hits to build a combo multiplier (up to x5).
- Avoid the black **bomb** balloons — hitting one penalizes your score and time.

### Scoring

| Balloon | Points |
| ------- | ------ |
| Green   | 4      |
| Red     | 8      |
| Yellow  | 16     |
| White   | 20     |
| Black (bomb) | penalty (score + time) |

Combo multiplier = consecutive hit streak, capped at x5. The combo resets on a miss
or after a few seconds without a hit.

## Controls

| Key / Input | Action            |
| ----------- | ----------------- |
| `W A S D`   | Move              |
| Mouse       | Aim (look)        |
| `Space`     | Jump              |
| `F`         | Shoot (hitscan)   |
| `R`         | Restart round     |
| `Esc`       | Toggle cursor lock |

A `+` crosshair marks the screen center; the hitscan ray fires through it and the
crosshair flashes on a hit. The HUD shows score (top-left), countdown (top-right),
and the active combo multiplier (center).

## Requirements

- [Rust](https://www.rust-lang.org/) (edition 2021)
- A GPU/driver supporting Bevy's renderer (Vulkan / Metal / DX12)
- Linux: install Bevy's system dependencies (e.g. `alsa`, `udev`, `libx11`) —
  see the [Bevy Linux setup guide](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md)

## Build & Run

```bash
cargo run
```

First build is slow (compiles Bevy). The `dev` profile is tuned for faster
iterative compiles (`opt-level = 1` for the crate, `3` for dependencies).

For a faster, optimized build:

```bash
cargo run --release
```

## Project Structure

| File              | Responsibility                                              |
| ----------------- | ----------------------------------------------------------- |
| `src/main.rs`     | App setup, plugins, HUD, score/timer/combo resources, systems |
| `src/player.rs`   | Kinematic capsule + camera, WASD/jump movement, mouse-look  |
| `src/terrain.rs`  | Procedural noise-based hilly terrain + staircase            |
| `src/scenery.rs`  | Decorative objects (trees, grass)                           |
| `src/balloons.rs` | Spawning, drift/bob motion, and respawn of balloon targets  |
| `src/shooting.rs` | Hitscan ray, hit detection, scoring, combo logic            |
| `src/effects.rs`  | Explosion effects on balloon hits                           |

## Dependencies

- `bevy` 0.18.1 — engine
- `bevy_rapier3d` 0.34 — 3D physics & collision (debug render enabled)
- `noise` 0.9 — procedural terrain heightmap
- `rand` 0.8 — random balloon placement

## Status

Prototype — core mechanic validation. Lighting and assets are intentionally minimal.
