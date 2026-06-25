//! Shooting: F fires a hitscan ray from the camera. If it hits a balloon, the balloon
//! despawns and its points are added to the score (logged to the console).

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::balloons::Balloon;
use crate::effects::spawn_explosion;
use crate::player::{Player, PlayerCamera};
use crate::{Combo, GameTimer, HitFlash, Score, MAX_MULTIPLIER};

const MAX_RANGE: f32 = 1000.0;
const COMBO_DURATION: f32 = 3.0;

const BOMB_TIME_PENALTY: f32 = 5.0;

pub fn shoot(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut timer: ResMut<GameTimer>,
    rapier_context: ReadRapierContext,
    camera_q: Query<&GlobalTransform, With<PlayerCamera>>,
    player_q: Query<Entity, With<Player>>,
    balloons: Query<&Balloon>,
    mut score: ResMut<Score>,
    mut combo: ResMut<Combo>,
    mut hit_flash: ResMut<HitFlash>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) -> Result<()> {
    // No scoring once the round is over.
    if !timer.running || !keyboard.just_pressed(KeyCode::KeyF) {
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

    let context = rapier_context.single()?;
    let mut hit_balloon = false;
    context.with_query_pipeline(filter, |pipeline| {
        if let Some((entity, toi)) = pipeline.cast_ray(origin, direction, MAX_RANGE, true) {
            if let Ok(balloon) = balloons.get(entity) {
                hit_balloon = true;
                hit_flash.t = 0.15;
                let hit_point = origin + direction * toi;

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
                        &mut commands,
                        &mut meshes,
                        &mut materials,
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
                    spawn_explosion(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        hit_point,
                        balloon.color,
                    );
                }
                commands.entity(entity).despawn();
            }
        }
    });

    // A shot that hits nothing (or non-balloon) breaks the combo.
    if !hit_balloon {
        combo.streak = 0;
        combo.decay = 0.0;
    }

    Ok(())
}
