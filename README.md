# first-person

A first-person shooter prototype built with [Bevy](https://bevyengine.org/) `0.18`
and [`bevy_rapier3d`](https://github.com/dimforge/bevy_rapier) for physics. You walk
around a textured, procedurally generated hilly terrain — enclosed by stone walls
that follow the ground contour, dotted with layered fir trees, and wrapped in a sky
dome with atmospheric distance fog — and shoot floating "balloon" targets against
the clock for points.

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

| Key / Input  | Action             |
| ------------ | ------------------ |
| `W A S D`    | Move               |
| Mouse        | Aim (look)         |
| `Space`      | Jump               |
| Left click   | Shoot              |
| `F`          | Toggle fog (fades in/out) |
| `R`          | Restart round      |
| `Esc`        | Toggle cursor lock |

A `+` crosshair marks the screen center. Clicking fires a glowing projectile that
flies toward whatever the crosshair is aimed at (a hitscan ray picks the target);
the crosshair flashes when the projectile lands a hit. The HUD shows score
(top-left), countdown (top-right), and the active combo multiplier (center).

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
| `src/main.rs`     | App setup, plugins, HUD, score/timer/combo resources, sky dome, systems |
| `src/player.rs`   | Kinematic capsule + camera, WASD/jump movement, mouse-look, fog fade |
| `src/terrain.rs`  | Procedural noise-based hilly terrain, staircase, contour-following walls |
| `src/scenery.rs`  | Decorative objects (layered fir trees, grass)              |
| `src/balloons.rs` | Spawning, drift/bob motion, and respawn of balloon targets  |
| `src/shooting.rs` | Projectile firing/homing, hit detection, scoring, combo logic |
| `src/effects.rs`  | Explosion effects on balloon hits                           |
| `assets/`         | Textures: `ground.png`, `leaves.png`, `sky.png`, `stone.png` |

## Dependencies

- `bevy` 0.18.1 — engine
- `bevy_rapier3d` 0.34 — 3D physics & collision
- `noise` 0.9 — procedural terrain heightmap
- `rand` 0.8 — random balloon placement

## Status

Prototype — core mechanic validation. Lighting is intentionally minimal.
