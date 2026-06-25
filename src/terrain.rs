//! Procedural environment: a noise-based hilly terrain and a primitive staircase.
//!
//! The terrain render mesh is generated from fbm/Perlin noise. Its collider is built
//! as a `trimesh` from the *exact same* vertices/indices, which guarantees the visible
//! surface and the physics surface are identical (no heightfield index-convention
//! alignment risk). Stairs use primitive cuboid colliders.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use noise::{Fbm, NoiseFn, Perlin};

const GRID: usize = 64; // vertices per side
pub const SIZE: f32 = 80.0; // world span on X and Z, centered at origin
const AMP: f32 = 6.0; // height amplitude
const FREQ: f64 = 0.045; // noise sampling frequency
const TEXTURE_TILES: f32 = 12.0; // how many times the ground texture repeats per side

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
    asset_server: Res<AssetServer>,
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
            // Tile the texture across the terrain instead of stretching one copy.
            uvs.push([
                col as f32 / (GRID - 1) as f32 * TEXTURE_TILES,
                row as f32 / (GRID - 1) as f32 * TEXTURE_TILES,
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

    // Load the ground texture with a repeating sampler so the tiled UVs wrap.
    let ground_texture = asset_server.load_with_settings(
        "ground.png",
        |settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..default()
            });
        },
    );

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(ground_texture),
            perceptual_roughness: 0.95,
            ..default()
        })),
        Transform::default(),
        RigidBody::Fixed,
        terrain_collider,
    ));

    spawn_staircase(&mut commands, &mut meshes, &mut materials);
    spawn_walls(&mut commands, &mut meshes, &mut materials, &asset_server);
}

/// Solid walls around the terrain perimeter so the player can't walk/fall off the edge.
fn spawn_walls(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    let half = SIZE * 0.5;
    const THICKNESS: f32 = 1.0;
    const WALL_ABOVE: f32 = 3.0; // how far each segment rises above the local ground
    const BASE_Y: f32 = -12.0; // buried base, below the lowest terrain dip
    const SEGMENTS: usize = 64; // segments per edge; more = smoother contour

    // Stone texture with a repeating sampler; the UV transform tiles it so each
    // segment reads as masonry rather than a single stretched copy.
    let stone_texture = asset_server.load_with_settings(
        "stone.png",
        |settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..default()
            });
        },
    );
    let material = materials.add(StandardMaterial {
        base_color_texture: Some(stone_texture),
        perceptual_roughness: 0.95,
        ..default()
    });

    let seg_len = SIZE / SEGMENTS as f32;
    let seg_span = seg_len + THICKNESS; // overlap neighbors/corners so there are no gaps
    // One texture repeat per segment span: keeps texel density uniform on every face
    // and makes the horizontal tiling line up seamlessly between adjacent segments.
    let tile = seg_span;

    // Spawn one wall segment whose top follows the ground at `(x, z)`.
    let mut spawn_segment = |x: f32, z: f32, size_x: f32, size_z: f32| {
        let top = sample_height(x, z) + WALL_ABOVE;
        let height = top - BASE_Y;
        let center_y = (top + BASE_Y) * 0.5;
        commands.spawn((
            Mesh3d(meshes.add(tiled_box_mesh(Vec3::new(size_x, height, size_z), tile))),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(Vec3::new(x, center_y, z)),
            RigidBody::Fixed,
            Collider::cuboid(size_x * 0.5, height * 0.5, size_z * 0.5),
        ));
    };

    for i in 0..SEGMENTS {
        let t = -half + (i as f32 + 0.5) * seg_len; // segment center along the edge
        // East / west edges: thin in X, stepping along Z.
        spawn_segment(half + THICKNESS * 0.5, t, THICKNESS, seg_span);
        spawn_segment(-half - THICKNESS * 0.5, t, THICKNESS, seg_span);
        // North / south edges: thin in Z, stepping along X.
        spawn_segment(t, half + THICKNESS * 0.5, seg_span, THICKNESS);
        spawn_segment(t, -half - THICKNESS * 0.5, seg_span, THICKNESS);
    }
}

/// A cuboid mesh whose UVs are scaled per face by world size, so the texture keeps
/// a uniform texel density on every face (`tile` world units per texture repeat).
/// Bevy's built-in `Cuboid` maps each face 0..1, which stretches non-square faces.
fn tiled_box_mesh(size: Vec3, tile: f32) -> Mesh {
    let h = size * 0.5;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(24);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(24);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(24);
    let mut indices: Vec<u32> = Vec::with_capacity(36);

    // Append one face given its center, in-plane unit axes `u`/`v` (with u×v = outward
    // normal) and their half-extents. UVs span 0..(2*half/tile) for even tiling.
    let mut face = |center: Vec3, u: Vec3, v: Vec3, hu: f32, hv: f32| {
        let n = u.cross(v);
        let base = positions.len() as u32;
        let us = (2.0 * hu) / tile;
        let vs = (2.0 * hv) / tile;
        let corners = [
            center - u * hu - v * hv,
            center + u * hu - v * hv,
            center + u * hu + v * hv,
            center - u * hu + v * hv,
        ];
        let face_uvs = [[0.0, 0.0], [us, 0.0], [us, vs], [0.0, vs]];
        for i in 0..4 {
            positions.push(corners[i].to_array());
            normals.push([n.x, n.y, n.z]);
            uvs.push(face_uvs[i]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };

    let (x, y, z) = (Vec3::X, Vec3::Y, Vec3::Z);
    face(Vec3::new(h.x, 0.0, 0.0), -z, y, h.z, h.y); // +X
    face(Vec3::new(-h.x, 0.0, 0.0), z, y, h.z, h.y); // -X
    face(Vec3::new(0.0, h.y, 0.0), z, x, h.z, h.x); // +Y
    face(Vec3::new(0.0, -h.y, 0.0), x, z, h.x, h.z); // -Y
    face(Vec3::new(0.0, 0.0, h.z), x, y, h.x, h.y); // +Z
    face(Vec3::new(0.0, 0.0, -h.z), -x, y, h.x, h.y); // -Z

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
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
