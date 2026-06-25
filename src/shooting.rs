//! Shooting: left mouse click fires a projectile from the camera. A hitscan ray decides what the
//! shot is aimed at, then a visible bullet flies toward that point/target. Scoring,
//! the hit-marker flash, and the explosion all resolve when the bullet arrives.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::balloons::Balloon;
use crate::effects::spawn_explosion;
use crate::player::{Player, PlayerCamera};
use crate::{Combo, GameTimer, HitFlash, Score, MAX_MULTIPLIER};

const MAX_RANGE: f32 = 1000.0;
const COMBO_DURATION: f32 = 3.0;

const BOMB_TIME_PENALTY: f32 = 5.0;

/// Bullet travel speed (world units per second) and visual size.
const PROJECTILE_SPEED: f32 = 80.0;
const PROJECTILE_RADIUS: f32 = 0.12;
/// Max seconds a bullet lives before being culled (covers shots that hit nothing).
const PROJECTILE_LIFE: f32 = 3.0;
/// Distance at which a bullet counts as "arrived" at its destination.
const HIT_RADIUS: f32 = 0.9;

/// A bullet in flight. It homes toward `target` (the aimed balloon) while that
/// balloon is alive, otherwise it flies to the static `aim_point` and despawns.
#[derive(Component)]
pub struct Projectile {
    target: Option<Entity>,
    aim_point: Vec3,
    life: f32,
}

pub fn shoot(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    timer: Res<GameTimer>,
    rapier_context: ReadRapierContext,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    player_q: Query<Entity, With<Player>>,
    balloons: Query<&Balloon>,
    mut combo: ResMut<Combo>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) -> Result<()> {
    // No firing once the round is over.
    if !timer.running || !mouse_buttons.just_pressed(MouseButton::Left) {
        return Ok(());
    }
    let Ok(camera) = camera_q.single() else {
        return Ok(());
    };

    let origin = camera.translation();
    let direction = Vec3::from(camera.forward());

    let mut filter = QueryFilter::default();
    if let Ok(player) = player_q.single() {
        // Don't let the ray hit the player's own capsule.
        filter = filter.exclude_collider(player);
    }

    // Hitscan only decides what the shot is aimed at; the actual scoring happens
    // when the projectile reaches it (see `move_projectiles`).
    let context = rapier_context.single()?;
    let mut target: Option<Entity> = None;
    let mut aim_point = origin + direction * MAX_RANGE;
    context.with_query_pipeline(filter, |pipeline| {
        if let Some((entity, toi)) = pipeline.cast_ray(origin, direction, MAX_RANGE, true) {
            aim_point = origin + direction * toi;
            if balloons.get(entity).is_ok() {
                target = Some(entity);
            }
        }
    });

    // A shot not aimed at a balloon breaks the combo right away.
    if target.is_none() {
        combo.streak = 0;
        combo.decay = 0.0;
    }

    // Spawn the visible bullet, offset slightly so it appears to leave a muzzle.
    let muzzle = origin + direction * 0.6 + Vec3::from(camera.right()) * 0.2
        - Vec3::from(camera.up()) * 0.2;
    commands.spawn((
        Projectile {
            target,
            aim_point,
            life: PROJECTILE_LIFE,
        },
        Mesh3d(meshes.add(Sphere::new(PROJECTILE_RADIUS))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.85, 0.2),
            emissive: LinearRgba::from(Color::srgb(1.0, 0.7, 0.1)) * 6.0,
            ..default()
        })),
        Transform::from_translation(muzzle),
    ));

    Ok(())
}

/// Advance bullets toward their target/aim point; resolve the hit on arrival.
pub fn move_projectiles(
    time: Res<Time>,
    mut timer: ResMut<GameTimer>,
    mut score: ResMut<Score>,
    mut combo: ResMut<Combo>,
    mut hit_flash: ResMut<HitFlash>,
    mut projectiles: Query<(Entity, &mut Transform, &mut Projectile)>,
    balloon_tf: Query<&GlobalTransform, With<Balloon>>,
    balloons: Query<&Balloon>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    let step = PROJECTILE_SPEED * dt;

    for (entity, mut transform, mut proj) in &mut projectiles {
        // Home toward the live target balloon; fall back to the static aim point.
        let dest = proj
            .target
            .and_then(|t| balloon_tf.get(t).ok())
            .map(|gt| gt.translation())
            .unwrap_or(proj.aim_point);

        let to = dest - transform.translation;
        let dist = to.length();

        if dist <= step.max(HIT_RADIUS) {
            // Arrived: score the hit if the aimed balloon is still around.
            if let Some(t) = proj.target {
                if let Ok(balloon) = balloons.get(t) {
                    resolve_hit(
                        balloon,
                        dest,
                        &mut score,
                        &mut combo,
                        &mut timer,
                        &mut hit_flash,
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                    );
                    commands.entity(t).despawn();
                }
            }
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation += to / dist * step;
        proj.life -= dt;
        if proj.life <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Apply scoring, combo, flash, and explosion for a bullet hitting `balloon`.
#[allow(clippy::too_many_arguments)]
fn resolve_hit(
    balloon: &Balloon,
    hit_point: Vec3,
    score: &mut Score,
    combo: &mut Combo,
    timer: &mut GameTimer,
    hit_flash: &mut HitFlash,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    hit_flash.t = 0.15;

    if balloon.is_bomb {
        // Bomb: deduct points, dock time, break the combo, orange blast.
        score.0 = score.0.saturating_sub(balloon.points);
        combo.streak = 0;
        combo.decay = 0.0;
        timer.remaining = (timer.remaining - BOMB_TIME_PENALTY).max(0.0);
        info!(
            "BOMB! -{} pts, -{}s. Total: {}",
            balloon.points, BOMB_TIME_PENALTY, score.0
        );
        spawn_explosion(
            commands,
            meshes,
            materials,
            hit_point,
            Color::srgb(1.0, 0.45, 0.05),
        );
    } else {
        combo.streak += 1;
        combo.decay = COMBO_DURATION;
        let multiplier = combo.streak.min(MAX_MULTIPLIER);
        let awarded = balloon.points * multiplier;
        score.0 += awarded;
        info!(
            "Hit +{} (x{}) Total: {}  [streak {}]",
            awarded, multiplier, score.0, combo.streak
        );
        spawn_explosion(commands, meshes, materials, hit_point, balloon.color);
    }
}
