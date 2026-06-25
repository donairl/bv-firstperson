//! Decorative scenery scattered over the terrain using primitive meshes:
//! trees (cylinder trunk + cone foliage, with a trunk collider) and grass blades
//! (small green boxes, no collider). All placed on the terrain surface.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;

use crate::terrain::{sample_height, SIZE};

const TREES: usize = 32;
const GRASS: usize = 400;
const SPREAD: f32 = 0.9; // fraction of half-terrain used for placement

pub fn setup_scenery(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let half = SIZE * 0.5 * SPREAD;
    let mut rng = rand::thread_rng();

    // Shared meshes/materials so we don't allocate per instance.
    let trunk_mesh = meshes.add(Cylinder::new(0.25, 3.0));
    let blade_mesh = meshes.add(Cuboid::new(0.08, 0.5, 0.08));

    // Stacked cones of decreasing size give each tree a layered fir/"cemara" shape.
    // Each entry is (radius, height, base height in the trunk's local space); the
    // tiers overlap so there are no gaps between them.
    let foliage_tiers: [(f32, f32, f32); 4] = [
        (1.8, 2.2, 1.2),
        (1.4, 2.0, 2.1),
        (1.0, 1.8, 3.0),
        (0.6, 1.6, 3.9),
    ];
    // Pre-build each tier's mesh and the local Y of its center (base + half height).
    let tier_meshes: Vec<(Handle<Mesh>, f32)> = foliage_tiers
        .iter()
        .map(|&(radius, height, base)| (meshes.add(Cone { radius, height }), base + height * 0.5))
        .collect();

    let trunk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.40, 0.26, 0.13),
        perceptual_roughness: 1.0,
        ..default()
    });
    // Tree foliage uses the shared texture for its leaves.
    let foliage_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("leaves.png")),
        perceptual_roughness: 0.95,
        ..default()
    });
    let grass_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.55, 0.18),
        perceptual_roughness: 1.0,
        ..default()
    });

    // Trees: trunk (with collider) + cone foliage as a child.
    for _ in 0..TREES {
        let x = rng.gen_range(-half..half);
        let z = rng.gen_range(-half..half);
        let ground = sample_height(x, z);
        let trunk_h = 3.0;

        commands
            .spawn((
                Mesh3d(trunk_mesh.clone()),
                MeshMaterial3d(trunk_mat.clone()),
                Transform::from_xyz(x, ground + trunk_h * 0.5, z),
                RigidBody::Fixed,
                Collider::cylinder(trunk_h * 0.5, 0.25),
            ))
            .with_children(|parent| {
                // Foliage: stacked cones rising up the trunk's local +Y axis.
                for (mesh, center_y) in &tier_meshes {
                    parent.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(foliage_mat.clone()),
                        Transform::from_xyz(0.0, *center_y, 0.0),
                    ));
                }
            });
    }

    // Grass blades: small boxes, random yaw, no collider.
    for _ in 0..GRASS {
        let x = rng.gen_range(-half..half);
        let z = rng.gen_range(-half..half);
        let ground = sample_height(x, z);
        let yaw = rng.gen_range(0.0..std::f32::consts::TAU);

        commands.spawn((
            Mesh3d(blade_mesh.clone()),
            MeshMaterial3d(grass_mat.clone()),
            Transform::from_xyz(x, ground + 0.25, z).with_rotation(Quat::from_rotation_y(yaw)),
        ));
    }
}
