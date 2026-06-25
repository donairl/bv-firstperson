//! Procedural environment: a noise-based hilly terrain and a primitive staircase.
//!
//! The terrain render mesh is generated from fbm/Perlin noise. Its collider is built
//! as a `trimesh` from the *exact same* vertices/indices, which guarantees the visible
//! surface and the physics surface are identical (no heightfield index-convention
//! alignment risk). Stairs use primitive cuboid colliders.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use noise::{Fbm, NoiseFn, Perlin};

const GRID: usize = 64; // vertices per side
pub const SIZE: f32 = 80.0; // world span on X and Z, centered at origin
const AMP: f32 = 6.0; // height amplitude
const FREQ: f64 = 0.045; // noise sampling frequency

/// World-space terrain height at `(x, z)`. Mirrors the noise/mapping used to build
/// the mesh, so scattered scenery sits exactly on the surface.
pub fn sample_height(x: f32, z: f32) -> f32 {
    let fbm = Fbm::<Perlin>::new(42);
    let col = (x / SIZE + 0.5) * (GRID - 1) as f32;
    let row = (z / SIZE + 0.5) * (GRID - 1) as f32;
    fbm.get([col as f64 * FREQ, row as f64 * FREQ]) as f32 * AMP
}

pub fn setup_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let fbm = Fbm::<Perlin>::new(42);

    // Vertex grid. Index (row, col) -> row * GRID + col.
    let height_at = |row: usize, col: usize| -> f32 {
        fbm.get([col as f64 * FREQ, row as f64 * FREQ]) as f32 * AMP
    };

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(GRID * GRID);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(GRID * GRID);
    for row in 0..GRID {
        for col in 0..GRID {
            let x = (col as f32 / (GRID - 1) as f32 - 0.5) * SIZE;
            let z = (row as f32 / (GRID - 1) as f32 - 0.5) * SIZE;
            positions.push([x, height_at(row, col), z]);
            uvs.push([
                col as f32 / (GRID - 1) as f32,
                row as f32 / (GRID - 1) as f32,
            ]);
        }
    }

    let vindex = |row: usize, col: usize| (row * GRID + col) as u32;
    let mut indices: Vec<u32> = Vec::with_capacity((GRID - 1) * (GRID - 1) * 6);
    for row in 0..GRID - 1 {
        for col in 0..GRID - 1 {
            let tl = vindex(row, col);
            let tr = vindex(row, col + 1);
            let bl = vindex(row + 1, col);
            let br = vindex(row + 1, col + 1);
            indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    // Smooth per-vertex normals by accumulating face normals.
    let mut normals = vec![[0.0f32; 3]; positions.len()];
    for tri in indices.chunks(3) {
        let a = Vec3::from(positions[tri[0] as usize]);
        let b = Vec3::from(positions[tri[1] as usize]);
        let c = Vec3::from(positions[tri[2] as usize]);
        let n = (b - a).cross(c - a);
        for &i in tri {
            let v = &mut normals[i as usize];
            v[0] += n.x;
            v[1] += n.y;
            v[2] += n.z;
        }
    }
    for n in normals.iter_mut() {
        let v = Vec3::from(*n).normalize_or_zero();
        *n = [v.x, v.y, v.z];
    }

    // Collider from the identical geometry -> guaranteed visual/physics alignment.
    let collider_verts: Vec<Vec3> = positions.iter().map(|p| Vec3::from(*p)).collect();
    let collider_indices: Vec<[u32; 3]> =
        indices.chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
    let terrain_collider = Collider::trimesh(collider_verts, collider_indices)
        .expect("terrain trimesh should be valid");

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

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

    spawn_staircase(&mut commands, &mut meshes, &mut materials);
}

/// A staircase made of stacked cuboid steps that go up and then back down.
/// Each step rises 0.4 (< the controller's 0.5 autostep height) so it is climbable.
fn spawn_staircase(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    const STEPS: usize = 8;
    const STEP_RISE: f32 = 0.4;
    const STEP_DEPTH: f32 = 1.2;
    const STEP_WIDTH: f32 = 5.0;

    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.5, 0.42),
        perceptual_roughness: 0.9,
        ..default()
    });
    let base = Vec3::new(14.0, 0.0, 0.0);

    let mut spawn_step = |center: Vec3, top: f32| {
        // A solid block from y=0 up to `top`, so there are no gaps to fall through.
        let half = Vec3::new(STEP_WIDTH * 0.5, top * 0.5, STEP_DEPTH * 0.5);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(STEP_WIDTH, top, STEP_DEPTH))),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(Vec3::new(center.x, top * 0.5, center.z)),
            RigidBody::Fixed,
            Collider::cuboid(half.x, half.y, half.z),
        ));
    };

    // Ascending flight.
    for i in 0..STEPS {
        let top = STEP_RISE * (i as f32 + 1.0);
        let z = base.z + STEP_DEPTH * i as f32;
        spawn_step(Vec3::new(base.x, 0.0, z), top);
    }
    // Descending flight on the far side.
    for i in 0..STEPS {
        let top = STEP_RISE * (STEPS as f32 - i as f32);
        let z = base.z + STEP_DEPTH * (STEPS + i) as f32;
        spawn_step(Vec3::new(base.x, 0.0, z), top);
    }
}
