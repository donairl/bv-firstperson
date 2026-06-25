//! Player: a kinematic capsule with a child camera. Handles WASD + jump movement
//! (gravity applied manually), mouse-look (yaw on the body, pitch on the camera),
//! and Esc to toggle cursor lock.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_rapier3d::prelude::*;

/// Marker for the player body (holds yaw rotation + the character controller).
#[derive(Component)]
pub struct Player;

/// Marker for the first-person camera (child of [`Player`], holds pitch rotation).
#[derive(Component)]
pub struct PlayerCamera;

const MOVE_SPEED: f32 = 6.0;
const JUMP_SPEED: f32 = 9.0;
const GRAVITY: f32 = -20.0;
const MOUSE_SENS: f32 = 0.002;
const PITCH_LIMIT: f32 = 1.5620; // ~89.5 degrees — allows aiming nearly straight up/down

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
            Transform::from_xyz(0.0, 18.0, 0.0),
            // Visibility on the body so the camera child's visibility chain is consistent.
            Visibility::default(),
            Collider::capsule_y(0.5, 0.3),
            KinematicCharacterController {
                up: Vec3::Y,
                offset: CharacterLength::Absolute(0.01),
                slide: true,
                // Automatic stair-stepping: lets the player walk up the staircase.
                autostep: Some(CharacterAutostep {
                    max_height: CharacterLength::Absolute(0.5),
                    min_width: CharacterLength::Relative(0.5),
                    include_dynamic_bodies: false,
                }),
                // Slopes up to 45 deg are climbable (the hilly terrain).
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
                // Ambient fill light for this camera's view.
                AmbientLight {
                    brightness: 350.0,
                    ..default()
                },
            ));
        });
}

/// WASD horizontal movement (relative to body yaw) + manual gravity & jump.
pub fn player_move(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<
        (
            &mut KinematicCharacterController,
            &Transform,
            Option<&KinematicCharacterControllerOutput>,
        ),
        With<Player>,
    >,
    mut vertical_velocity: Local<f32>,
) {
    let Ok((mut controller, transform, output)) = query.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    let grounded = output.map(|o| o.grounded).unwrap_or(false);

    if grounded && *vertical_velocity < 0.0 {
        *vertical_velocity = 0.0;
    }
    if grounded && keyboard.just_pressed(KeyCode::Space) {
        *vertical_velocity = JUMP_SPEED;
    }
    *vertical_velocity += GRAVITY * dt;

    let mut input = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        input.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        input.z += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        input.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        input.x += 1.0;
    }

    // Rotate input by the body's yaw, flatten, then normalize.
    let mut movement = transform.rotation * input;
    movement.y = 0.0;
    movement = movement.normalize_or_zero() * MOVE_SPEED;
    movement.y = *vertical_velocity;

    controller.translation = Some(movement * dt);
}

/// Mouse-look: horizontal motion yaws the body, vertical motion pitches the camera.
pub fn mouse_look(
    mouse: Res<AccumulatedMouseMotion>,
    mut player: Query<&mut Transform, (With<Player>, Without<PlayerCamera>)>,
    mut camera: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    mut pitch: Local<f32>,
) {
    let delta = mouse.delta;
    if delta == Vec2::ZERO {
        return;
    }
    if let Ok(mut body) = player.single_mut() {
        body.rotate_y(-delta.x * MOUSE_SENS);
    }
    if let Ok(mut cam) = camera.single_mut() {
        *pitch = (*pitch - delta.y * MOUSE_SENS).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        cam.rotation = Quat::from_rotation_x(*pitch);
    }
}

/// Esc toggles cursor capture so you can free the mouse / refocus the window.
pub fn toggle_cursor(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }
    if let Ok(mut cursor) = cursor.single_mut() {
        match cursor.grab_mode {
            CursorGrabMode::Locked => {
                cursor.grab_mode = CursorGrabMode::None;
                cursor.visible = true;
            }
            _ => {
                cursor.grab_mode = CursorGrabMode::Locked;
                cursor.visible = false;
            }
        }
    }
}
