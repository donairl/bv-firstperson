# Tutorial: Building the `first-person` Balloon Shooter in Bevy

A step-by-step guide to building this first-person shooter from an empty Cargo
project. Engine: **Bevy 0.18**. Physics: **bevy_rapier3d 0.34**. You'll build a
player you can walk/jump/aim with, procedural terrain, scenery, floating balloon
targets, hitscan shooting, explosion effects, and a scored timed round with combos.

> Each step compiles on its own. Run `cargo run` after a step to see progress.

---

## Prerequisites

- Rust (edition 2021) — install via [rustup](https://rustup.rs/).
- A GPU/driver supporting Vulkan / Metal / DX12.
- Linux only: install Bevy's system deps (`alsa`, `udev`, `libx11`, …). See the
  [Bevy Linux setup guide](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md).

---

## Step 1 — Create the project

```bash
cargo new first-person
cd first-person
```

Edit `Cargo.toml` to add dependencies and a faster dev profile:

```toml
[package]
name = "first-person"
version = "0.1.0"
edition = "2021"

[dependencies]
bevy = "0.18.1"
bevy_rapier3d = { version = "0.34", features = ["debug-render-3d"] }
noise = "0.9"
rand = "0.8"

# Faster iterative compile for a prototype
[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3
```

- `bevy` — the engine (windowing, rendering, ECS, input).
- `bevy_rapier3d` — 3D physics, colliders, the character controller, raycasting.
  `debug-render-3d` draws collider wireframes (handy while building).
- `noise` — Perlin/fbm noise for the terrain heightmap.
- `rand` — random placement of balloons and scenery.

The profile tweak compiles **your** crate fast (`opt-level = 1`) but dependencies
optimized (`3`), so Bevy stays fast at runtime without slowing rebuilds.

First build downloads + compiles Bevy — slow once, cached after.

---

## Step 2 — A window with a light (`main.rs`)

Start `src/main.rs` with the Bevy + Rapier plugins and a single directional light
("sun") so anything we add later is visible.

```rust
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(Startup, setup_environment)
        .run();
}

/// Sun so the scene is readable without polished lighting.
fn setup_environment(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(20.0, 40.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
```

`cargo run` → an empty window. The ECS model: **systems** (functions) run each frame
or at startup, operating on **components** spawned onto **entities**. `Commands`
queues entity spawns.

---

## Step 3 — The player: kinematic capsule + camera (`player.rs`)

Create `src/player.rs`. The player is a **kinematic character controller** capsule
with a first-person camera as a child. Yaw (left/right) rotates the body; pitch
(up/down) rotates the camera.

Key pieces:

```rust
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_rapier3d::prelude::*;

#[derive(Component)]
pub struct Player;        // body: yaw + character controller

#[derive(Component)]
pub struct PlayerCamera;  // child camera: pitch

const MOVE_SPEED: f32 = 6.0;
const JUMP_SPEED: f32 = 9.0;
const GRAVITY: f32 = -20.0;
const MOUSE_SENS: f32 = 0.002;
const PITCH_LIMIT: f32 = 1.5620; // ~89.5° — aim nearly straight up/down
```

**Spawn** (`setup_player`): lock + hide the cursor, spawn the `Player` capsule with a
`KinematicCharacterController`, and attach a `Camera3d` child:

```rust
pub fn setup_player(
    mut commands: Commands,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if let Ok(mut cursor) = cursor.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }

    commands
        .spawn((
            Player,
            Transform::from_xyz(0.0, 18.0, 0.0), // start high, fall onto terrain
            Visibility::default(),
            Collider::capsule_y(0.5, 0.3),
            KinematicCharacterController {
                up: Vec3::Y,
                offset: CharacterLength::Absolute(0.01),
                slide: true,
                autostep: Some(CharacterAutostep {
                    max_height: CharacterLength::Absolute(0.5), // climb stairs ≤ 0.5
                    min_width: CharacterLength::Relative(0.5),
                    include_dynamic_bodies: false,
                }),
                max_slope_climb_angle: 45.0_f32.to_radians(),
                min_slope_slide_angle: 30.0_f32.to_radians(),
                snap_to_ground: Some(CharacterLength::Absolute(0.3)),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                PlayerCamera,
                Camera3d::default(),
                Transform::from_xyz(0.0, 0.5, 0.0),
                AmbientLight { brightness: 350.0, ..default() },
            ));
        });
}
```

`autostep`/`max_slope_climb_angle`/`snap_to_ground` are what make stairs and hills
walkable later.

**Move** (`player_move`): read WASD, apply manual gravity + jump using a `Local<f32>`
for vertical velocity, rotate horizontal input by the body's yaw, and feed the
controller a per-frame translation:

```rust
pub fn player_move(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(
        &mut KinematicCharacterController,
        &Transform,
        Option<&KinematicCharacterControllerOutput>,
    ), With<Player>>,
    mut vertical_velocity: Local<f32>,
) {
    let Ok((mut controller, transform, output)) = query.single_mut() else { return; };
    let dt = time.delta_secs();
    let grounded = output.map(|o| o.grounded).unwrap_or(false);

    if grounded && *vertical_velocity < 0.0 { *vertical_velocity = 0.0; }
    if grounded && keyboard.just_pressed(KeyCode::Space) { *vertical_velocity = JUMP_SPEED; }
    *vertical_velocity += GRAVITY * dt;

    let mut input = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) { input.z -= 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { input.z += 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { input.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { input.x += 1.0; }

    let mut movement = transform.rotation * input;
    movement.y = 0.0;
    movement = movement.normalize_or_zero() * MOVE_SPEED;
    movement.y = *vertical_velocity;

    controller.translation = Some(movement * dt);
}
```

**Look** (`mouse_look`): horizontal mouse delta yaws the body, vertical pitches the
camera (clamped). **Toggle** (`toggle_cursor`): `Esc` flips cursor lock. (See
`src/player.rs` for both — they're short.)

Wire it into `main.rs`:

```rust
mod player;
// ...
.add_systems(Startup, (setup_environment, player::setup_player))
.add_systems(Update, (
    player::player_move,
    player::mouse_look,
    player::toggle_cursor,
))
```

`cargo run` → you fall and stand on... nothing yet. Add ground next.

---

## Step 4 — Procedural terrain + stairs (`terrain.rs`)

Create `src/terrain.rs`. Build a hilly mesh from fbm/Perlin noise, and crucially
build the **collider from the exact same vertices** so what you see is what you
collide with.

Constants and a shared height function:

```rust
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use noise::{Fbm, NoiseFn, Perlin};

const GRID: usize = 64;       // vertices per side
pub const SIZE: f32 = 80.0;   // world span on X/Z, centered at origin
const AMP: f32 = 6.0;         // height amplitude
const FREQ: f64 = 0.045;      // noise frequency

/// World height at (x, z) — reused by scenery so it sits on the surface.
pub fn sample_height(x: f32, z: f32) -> f32 {
    let fbm = Fbm::<Perlin>::new(42);
    let col = (x / SIZE + 0.5) * (GRID - 1) as f32;
    let row = (z / SIZE + 0.5) * (GRID - 1) as f32;
    fbm.get([col as f64 * FREQ, row as f64 * FREQ]) as f32 * AMP
}
```

`setup_terrain` builds the grid: for each `(row, col)` push a vertex position, a UV,
build two triangles per quad into an index buffer, accumulate smooth per-vertex
normals from face normals, then make:

- a **render mesh** (`PrimitiveTopology::TriangleList` with positions/normals/UVs),
- a **trimesh collider** from the same `positions` + `indices`.

```rust
let terrain_collider = Collider::trimesh(collider_verts, collider_indices)
    .expect("terrain trimesh should be valid");

commands.spawn((
    Mesh3d(meshes.add(mesh)),
    MeshMaterial3d(materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.55, 0.25),
        perceptual_roughness: 0.95,
        ..default()
    })),
    Transform::default(),
    RigidBody::Fixed,
    terrain_collider,
));
```

Then `spawn_staircase`: 8 ascending + 8 descending stacked cuboid steps, each rising
`0.4` (below the controller's `0.5` autostep height so they're climbable). Each step
is a solid block from `y=0` to its top with a matching `Collider::cuboid` — no gaps
to fall through.

Wire in `main.rs`: `mod terrain;` and add `terrain::setup_terrain` to `Startup`.

`cargo run` → you land on green hills and can walk up the staircase.

> **Why trimesh from the same geometry?** A heightfield collider has its own index
> convention; rebuilding from the mesh's exact verts removes any visual/physics
> misalignment risk.

---

## Step 5 — Scenery: trees and grass (`scenery.rs`)

Create `src/scenery.rs`. Scatter primitive-mesh decoration on the terrain using
`sample_height` so everything sits on the surface.

```rust
use crate::terrain::{sample_height, SIZE};

const TREES: usize = 32;
const GRASS: usize = 400;
const SPREAD: f32 = 0.9; // fraction of half-terrain used for placement
```

`setup_scenery`:
- Create **shared** meshes/materials once (trunk cylinder, foliage cone, grass blade
  cuboid) so instances don't reallocate.
- Trees: spawn a trunk `Cylinder` with a `Collider::cylinder` at
  `ground + trunk_h*0.5`, and add a `Cone` foliage as a **child** offset up its
  local +Y.
- Grass: 400 small green `Cuboid` blades with random yaw, **no collider**.

```rust
let ground = sample_height(x, z);
commands.spawn((
    Mesh3d(trunk_mesh.clone()),
    MeshMaterial3d(trunk_mat.clone()),
    Transform::from_xyz(x, ground + trunk_h * 0.5, z),
    RigidBody::Fixed,
    Collider::cylinder(trunk_h * 0.5, 0.25),
))
.with_children(|parent| {
    parent.spawn((
        Mesh3d(foliage_mesh.clone()),
        MeshMaterial3d(foliage_mat.clone()),
        Transform::from_xyz(0.0, trunk_h * 0.5 + 1.4, 0.0),
    ));
});
```

Wire in `main.rs`: `mod scenery;` + `scenery::setup_scenery` in `Startup`.

---

## Step 6 — Balloon targets (`balloons.rs`)

Create `src/balloons.rs`. Floating sphere targets, kept topped up to a fixed count.

Components + constants:

```rust
#[derive(Component)]
pub struct Balloon { pub points: u32, pub color: Color, pub is_bomb: bool }

#[derive(Component)]
pub struct BalloonMotion {
    drift: Vec3,    // horizontal velocity, bounces at edges
    base_y: f32,    // bob center height
    bob_amp: f32,
    bob_speed: f32,
    phase: f32,     // per-balloon offset so they don't sync
}

const TARGET_COUNT: usize = 20;
const AREA: f32 = 35.0; // half-extent for X/Z placement
const RADIUS: f32 = 0.8;
pub const BOMB_PENALTY: u32 = 25;
```

`random_kind` decides the type: ~18% are **bombs** (black, penalty). Otherwise
green=4, red=8, yellow=16, white=20.

```rust
fn random_kind(rng: &mut impl Rng) -> (u32, Color, bool) {
    if rng.gen_bool(0.18) {
        return (BOMB_PENALTY, Color::srgb(0.05, 0.05, 0.05), true);
    }
    match rng.gen_range(0..4) {
        0 => (4,  Color::srgb(0.1, 0.8, 0.1), false),
        1 => (8,  Color::srgb(0.9, 0.1, 0.1), false),
        2 => (16, Color::srgb(0.95, 0.9, 0.1), false),
        _ => (20, Color::srgb(1.0, 1.0, 1.0), false),
    }
}
```

`spawn_one` places a balloon at a random spot with random drift + bob, an emissive
material, and — importantly — a **kinematic** rigid body + ball collider so we can
script its motion while the collider follows for accurate raycasts:

```rust
RigidBody::KinematicPositionBased,
Collider::ball(RADIUS),
```

Three systems:
- `setup_balloons` — spawn `TARGET_COUNT` at startup.
- `move_balloons` — drift horizontally (reflect at `±AREA`), bob vertically with
  `sin((t + phase) * bob_speed) * bob_amp`.
- `balloon_respawn` — each frame, refill back up to `TARGET_COUNT` after hits.

Wire in `main.rs`: `mod balloons;`, add `balloons::setup_balloons` to `Startup`, and
`balloons::move_balloons` + `balloons::balloon_respawn` to `Update`.

`cargo run` → 20 glowing balloons drift and bob overhead.

---

## Step 7 — Explosion effect (`effects.rs`)

Create `src/effects.rs`. A cheap particle burst from primitive spheres.

```rust
#[derive(Component)]
pub struct ExplosionParticle { velocity: Vec3, life: f32, max_life: f32 }

const PARTICLES: usize = 16;
const PARTICLE_GRAVITY: f32 = -9.0;
```

`spawn_explosion(commands, meshes, materials, position, color)` spawns 16 small
emissive sphere fragments flying in random directions at random speeds, tinted to the
popped balloon. `update_explosions` advances them: apply gravity, move, shrink scale
by remaining-life ratio, despawn at `life <= 0`.

```rust
pub fn update_explosions(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Transform, &mut ExplosionParticle)>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut particle) in &mut particles {
        particle.life -= dt;
        if particle.life <= 0.0 { commands.entity(entity).despawn(); continue; }
        particle.velocity.y += PARTICLE_GRAVITY * dt;
        transform.translation += particle.velocity * dt;
        let ratio = (particle.life / particle.max_life).clamp(0.0, 1.0);
        transform.scale = Vec3::splat(ratio);
    }
}
```

Wire in `main.rs`: `mod effects;` + `effects::update_explosions` in `Update`.

---

## Step 8 — Game state resources (`main.rs`)

Add the scoring/round state. Resources are global singletons in the ECS.

```rust
#[derive(Resource, Default)]
pub struct Score(pub u32);

#[derive(Resource)]
pub struct GameTimer { pub remaining: f32, pub running: bool }
impl Default for GameTimer {
    fn default() -> Self { Self { remaining: ROUND_SECONDS, running: true } }
}
const ROUND_SECONDS: f32 = 60.0;

#[derive(Resource, Default)]
pub struct Combo { pub streak: u32, pub decay: f32 }
pub const MAX_MULTIPLIER: u32 = 5;

#[derive(Resource, Default)]
pub struct HitFlash { pub t: f32 } // crosshair flash countdown
```

Register them in `main`:

```rust
.init_resource::<Score>()
.init_resource::<GameTimer>()
.init_resource::<Combo>()
.init_resource::<HitFlash>()
```

---

## Step 9 — Shooting: hitscan ray + scoring (`shooting.rs`)

Create `src/shooting.rs`. On `F`, cast a ray from the camera through the crosshair.

```rust
const MAX_RANGE: f32 = 1000.0;
const COMBO_DURATION: f32 = 3.0;
const BOMB_TIME_PENALTY: f32 = 5.0;
```

`shoot` returns `Result<()>` (Bevy 0.18 supports fallible systems). Logic:

1. Bail if the round is over or `F` wasn't just pressed.
2. Take the camera's world position + forward as ray origin/direction.
3. Exclude the player's own collider from the ray.
4. Cast via Rapier's query pipeline; if it hits a `Balloon` entity:
   - **Bomb** → subtract points (`saturating_sub`), dock 5s, reset combo, orange
     blast.
   - **Normal** → `streak += 1`, refresh combo decay, award
     `points * min(streak, MAX_MULTIPLIER)`, spawn a colored blast.
   - Despawn the balloon and flash the crosshair (`hit_flash.t = 0.15`).
5. A shot that hits nothing/non-balloon resets the combo.

```rust
let context = rapier_context.single()?;
let mut hit_balloon = false;
context.with_query_pipeline(filter, |pipeline| {
    if let Some((entity, toi)) = pipeline.cast_ray(origin, direction, MAX_RANGE, true) {
        if let Ok(balloon) = balloons.get(entity) {
            hit_balloon = true;
            hit_flash.t = 0.15;
            let hit_point = origin + direction * toi;
            if balloon.is_bomb {
                score.0 = score.0.saturating_sub(balloon.points);
                combo.streak = 0; combo.decay = 0.0;
                timer.remaining = (timer.remaining - BOMB_TIME_PENALTY).max(0.0);
                spawn_explosion(&mut commands, &mut meshes, &mut materials,
                                hit_point, Color::srgb(1.0, 0.45, 0.05));
            } else {
                combo.streak += 1; combo.decay = COMBO_DURATION;
                let multiplier = combo.streak.min(MAX_MULTIPLIER);
                score.0 += balloon.points * multiplier;
                spawn_explosion(&mut commands, &mut meshes, &mut materials,
                                hit_point, balloon.color);
            }
            commands.entity(entity).despawn();
        }
    }
});
if !hit_balloon { combo.streak = 0; combo.decay = 0.0; }
```

Wire in `main.rs`: `mod shooting;` + `shooting::shoot` in `Update`.

`cargo run` → aim, press `F`, balloons pop with bursts and your score climbs (watch
the console logs).

---

## Step 10 — HUD, crosshair, timer, combo, restart (`main.rs`)

Final polish. Add UI nodes and the systems that drive them.

**Crosshair** (`setup_crosshair`): a full-screen centered `Node` holding a `+` text,
marked `CrosshairText`. The ray fires through this point.

**HUD** (`setup_hud`): a `ScoreText` (top-left), `TimeText` (top-right), and a
centered `ComboText` (hidden below streak 2). Use marker components:

```rust
#[derive(Component)] struct ScoreText;
#[derive(Component)] struct TimeText;
#[derive(Component)] struct CrosshairText;
#[derive(Component)] struct ComboText;
```

**Systems:**
- `tick_timer` — count `remaining` down while `running`; stop at 0.
- `update_hud` — refresh each text only when its resource `is_changed()`; show
  `COMBO x{n}` at streak ≥ 2, `"TIME UP! (R)"` when stopped.
- `combo_decay` — drop the streak to 0 if no hit within `COMBO_DECAY` seconds.
- `crosshair_feedback` — while `HitFlash.t > 0`, color the `+` yellow and enlarge it.
- `restart_round` — `R` resets score, combo, and timer.

Register everything. The final `main` looks like:

```rust
.add_systems(Startup, (
    setup_environment, setup_crosshair, setup_hud,
    terrain::setup_terrain, scenery::setup_scenery,
    player::setup_player, balloons::setup_balloons,
))
.add_systems(Update, (
    player::player_move, player::mouse_look, player::toggle_cursor,
    shooting::shoot,
    balloons::move_balloons, balloons::balloon_respawn,
    effects::update_explosions,
    tick_timer, update_hud, restart_round, combo_decay, crosshair_feedback,
))
```

> A `Startup`/`Update` tuple may exceed Bevy's system-count-per-tuple limit; if so,
> split into multiple `.add_systems(...)` calls or nest tuples.

`cargo run` → full game: walk the hills, aim with the mouse, pop balloons for a
combo-multiplied score, dodge bombs, beat the 60s clock, press `R` to replay.

---

## Module map (final)

| File              | Built in step | Responsibility                                  |
| ----------------- | ------------- | ----------------------------------------------- |
| `src/main.rs`     | 2, 8, 10      | App setup, resources, HUD, timer/combo systems  |
| `src/player.rs`   | 3             | Capsule + camera, movement, look, cursor        |
| `src/terrain.rs`  | 4             | Noise terrain mesh + matching collider, stairs  |
| `src/scenery.rs`  | 5             | Trees + grass on the surface                    |
| `src/balloons.rs` | 6             | Spawn, drift/bob, respawn targets               |
| `src/effects.rs`  | 7             | Explosion particle burst                        |
| `src/shooting.rs` | 9             | Hitscan ray, hit detection, scoring/combo       |

---

## Next ideas

- Sound effects on hit / bomb / round-end.
- A start menu and game-over screen (Bevy states).
- Balloon variety: shrinking, faster drifters, moving bomb clusters.
- Save the high score to disk.
- Replace primitive meshes with imported glTF assets.
