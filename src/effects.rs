//! Cheap explosion effect built from primitive meshes: a burst of small emissive
//! sphere fragments that fly outward, fall under gravity, shrink, and despawn.

use bevy::prelude::*;
use rand::Rng;

/// One fragment of an explosion. Advanced and culled by [`update_explosions`].
#[derive(Component)]
pub struct ExplosionParticle {
    velocity: Vec3,
    life: f32,     // remaining seconds
    max_life: f32, // initial life, used for shrink ratio
}

const PARTICLES: usize = 16;
const PARTICLE_GRAVITY: f32 = -9.0;

/// Spawn a burst of fragments at `position`, tinted to match the popped balloon.
pub fn spawn_explosion(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    color: Color,
) {
    let mut rng = rand::thread_rng();
    let mesh = meshes.add(Sphere::new(0.18));
    let material = materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::from(color) * 3.0,
        ..default()
    });

    for _ in 0..PARTICLES {
        let dir = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(0.1..1.0),
            rng.gen_range(-1.0..1.0),
        )
        .normalize_or_zero();
        let speed = rng.gen_range(3.0..7.0);
        let life = rng.gen_range(0.4..0.8);

        commands.spawn((
            ExplosionParticle {
                velocity: dir * speed,
                life,
                max_life: life,
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(position),
        ));
    }
}

/// Move fragments, apply gravity, shrink them over their lifetime, then despawn.
pub fn update_explosions(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Transform, &mut ExplosionParticle)>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut particle) in &mut particles {
        particle.life -= dt;
        if particle.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        particle.velocity.y += PARTICLE_GRAVITY * dt;
        let step = particle.velocity * dt;
        transform.translation += step;
        let ratio = (particle.life / particle.max_life).clamp(0.0, 1.0);
        transform.scale = Vec3::splat(ratio);
    }
}
