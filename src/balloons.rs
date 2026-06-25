//! Balloons: floating sphere targets worth points. Spawned at random positions and
//! kept topped up to a fixed count so the player always has something to shoot.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;

/// A shootable target. `points` is awarded (or, for a bomb, deducted) when hit;
/// `color` drives the explosion. Bombs are the black balloons — shooting one penalizes.
#[derive(Component)]
pub struct Balloon {
    pub points: u32,
    pub color: Color,
    pub is_bomb: bool,
}

/// Drift + bob motion applied to a balloon each frame by [`move_balloons`].
#[derive(Component)]
pub struct BalloonMotion {
    drift: Vec3,    // horizontal velocity (x, 0, z), bounces at the area edges
    base_y: f32,    // center height the bob oscillates around
    bob_amp: f32,   // vertical bob amplitude
    bob_speed: f32, // vertical bob frequency
    phase: f32,     // per-balloon phase offset so they don't bob in sync
}

const TARGET_COUNT: usize = 20;
const AREA: f32 = 35.0; // half-extent for random X/Z placement
const RADIUS: f32 = 0.8;

/// Points deducted for shooting a bomb balloon.
pub const BOMB_PENALTY: u32 = 25;

/// Returns `(points, color, is_bomb)`. ~18% of balloons are bombs (black).
/// Normal table: green=4, red=8, yellow=16, white=20.
fn random_kind(rng: &mut impl Rng) -> (u32, Color, bool) {
    if rng.gen_bool(0.18) {
        return (BOMB_PENALTY, Color::srgb(0.05, 0.05, 0.05), true); // bomb (black)
    }
    match rng.gen_range(0..4) {
        0 => (4, Color::srgb(0.1, 0.8, 0.1), false),   // green
        1 => (8, Color::srgb(0.9, 0.1, 0.1), false),   // red
        2 => (16, Color::srgb(0.95, 0.9, 0.1), false), // yellow
        _ => (20, Color::srgb(1.0, 1.0, 1.0), false),  // white
    }
}

fn spawn_one(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    rng: &mut impl Rng,
) {
    let (points, color, is_bomb) = random_kind(rng);
    let x = rng.gen_range(-AREA..AREA);
    let z = rng.gen_range(-AREA..AREA);
    let y = rng.gen_range(6.0..18.0);

    // Random horizontal drift direction + speed, and an independent bob.
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let speed = rng.gen_range(1.5..4.5);
    let drift = Vec3::new(angle.cos() * speed, 0.0, angle.sin() * speed);

    commands.spawn((
        Balloon {
            points,
            color,
            is_bomb,
        },
        BalloonMotion {
            drift,
            base_y: y,
            bob_amp: rng.gen_range(0.4..1.0),
            bob_speed: rng.gen_range(1.0..2.5),
            phase: rng.gen_range(0.0..std::f32::consts::TAU),
        },
        Mesh3d(meshes.add(Sphere::new(RADIUS))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: LinearRgba::from(color) * 0.4,
            ..default()
        })),
        Transform::from_xyz(x, y, z),
        // Kinematic so we script its motion while the collider still follows it
        // (keeps the hitscan raycast accurate on moving targets).
        RigidBody::KinematicPositionBased,
        Collider::ball(RADIUS),
    ));
}

pub fn setup_balloons(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();
    for _ in 0..TARGET_COUNT {
        spawn_one(&mut commands, &mut meshes, &mut materials, &mut rng);
    }
}

/// Drift balloons horizontally (bouncing at the area edges) and bob them vertically.
pub fn move_balloons(time: Res<Time>, mut query: Query<(&mut Transform, &mut BalloonMotion)>) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    for (mut transform, mut motion) in &mut query {
        // Horizontal drift with reflection at the placement bounds.
        transform.translation.x += motion.drift.x * dt;
        transform.translation.z += motion.drift.z * dt;
        if transform.translation.x.abs() > AREA {
            transform.translation.x = transform.translation.x.clamp(-AREA, AREA);
            motion.drift.x = -motion.drift.x;
        }
        if transform.translation.z.abs() > AREA {
            transform.translation.z = transform.translation.z.clamp(-AREA, AREA);
            motion.drift.z = -motion.drift.z;
        }
        // Vertical bob around the base height.
        transform.translation.y =
            motion.base_y + ((t + motion.phase) * motion.bob_speed).sin() * motion.bob_amp;
    }
}

/// Keep the balloon population topped up to `TARGET_COUNT` after hits.
pub fn balloon_respawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    balloons: Query<(), With<Balloon>>,
) {
    let count = balloons.iter().count();
    if count >= TARGET_COUNT {
        return;
    }
    let mut rng = rand::thread_rng();
    for _ in 0..(TARGET_COUNT - count) {
        spawn_one(&mut commands, &mut meshes, &mut materials, &mut rng);
    }
}
