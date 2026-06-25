//! First-person shooter prototype — core mechanic validation.
//!
//! Controls: WASD move, Space jump, mouse aim, F shoot, Esc toggle cursor lock.
//! Shoot floating sphere "balloons" for points: green=4, red=8, yellow=16, white=20.
//! Procedural hilly terrain (noise) + a staircase; physics/collision via bevy_rapier3d.

mod balloons;
mod effects;
mod player;
mod scenery;
mod shooting;
mod terrain;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/// Cumulative player score, shown in the HUD and logged on every hit.
#[derive(Resource, Default)]
pub struct Score(pub u32);

/// Marker for the on-screen score readout.
#[derive(Component)]
struct ScoreText;

/// Marker for the on-screen countdown readout.
#[derive(Component)]
struct TimeText;

/// Round countdown. Scoring is disabled once it reaches zero; press R to restart.
#[derive(Resource)]
pub struct GameTimer {
    pub remaining: f32,
    pub running: bool,
}

impl Default for GameTimer {
    fn default() -> Self {
        Self {
            remaining: ROUND_SECONDS,
            running: true,
        }
    }
}

const ROUND_SECONDS: f32 = 60.0;

/// Consecutive-hit streak. The score multiplier is `streak.min(MAX_MULTIPLIER)`.
/// Resets on a miss or after `COMBO_DECAY` seconds without a hit.
#[derive(Resource, Default)]
pub struct Combo {
    pub streak: u32,
    pub decay: f32, // seconds of combo left before it resets
}

/// Countdown (seconds) for the crosshair hit-marker flash.
#[derive(Resource, Default)]
pub struct HitFlash {
    pub t: f32,
}

pub const MAX_MULTIPLIER: u32 = 5;

/// Marker for the center "+" crosshair text (flashes on a hit).
#[derive(Component)]
struct CrosshairText;

/// Marker for the combo-multiplier HUD text.
#[derive(Component)]
struct ComboText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .init_resource::<Score>()
        .init_resource::<GameTimer>()
        .init_resource::<Combo>()
        .init_resource::<HitFlash>()
        .add_systems(
            Startup,
            (
                setup_environment,
                setup_crosshair,
                setup_hud,
                terrain::setup_terrain,
                scenery::setup_scenery,
                player::setup_player,
                balloons::setup_balloons,
            ),
        )
        .add_systems(
            Update,
            (
                player::player_move,
                player::mouse_look,
                player::toggle_cursor,
                shooting::shoot,
                balloons::move_balloons,
                balloons::balloon_respawn,
                effects::update_explosions,
                tick_timer,
                update_hud,
                restart_round,
                combo_decay,
                crosshair_feedback,
            ),
        )
        .run();
}

/// Sun so the scene is readable without polished lighting. (Ambient fill is added
/// as a component on the camera in `player::setup_player`.)
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

/// Fixed "+" crosshair at the screen center; the hitscan ray fires through this point.
fn setup_crosshair(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                CrosshairText,
                Text::new("+"),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Score (top-left) and countdown (top-right) HUD readouts.
fn setup_hud(mut commands: Commands) {
    commands.spawn((
        ScoreText,
        Text::new("Score: 0"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(14.0),
            ..default()
        },
    ));
    commands.spawn((
        TimeText,
        Text::new("Time: 60"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(14.0),
            ..default()
        },
    ));
    // Combo multiplier, centered just below the crosshair (hidden at streak < 2).
    commands.spawn((
        ComboText,
        Text::new(""),
        TextFont {
            font_size: 36.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.7, 0.1)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(56.0),
            left: Val::Percent(0.0),
            right: Val::Percent(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
}

/// Count the round down; stop at zero (scoring is gated on `running` in `shoot`).
fn tick_timer(time: Res<Time>, mut timer: ResMut<GameTimer>) {
    if !timer.running {
        return;
    }
    timer.remaining -= time.delta_secs();
    if timer.remaining <= 0.0 {
        timer.remaining = 0.0;
        timer.running = false;
        info!("Time up!");
    }
}

/// Refresh the HUD text from the current score, timer, and combo state.
fn update_hud(
    score: Res<Score>,
    timer: Res<GameTimer>,
    combo: Res<Combo>,
    mut score_text: Query<&mut Text, (With<ScoreText>, Without<TimeText>, Without<ComboText>)>,
    mut time_text: Query<&mut Text, (With<TimeText>, Without<ScoreText>, Without<ComboText>)>,
    mut combo_text: Query<&mut Text, (With<ComboText>, Without<ScoreText>, Without<TimeText>)>,
) {
    if score.is_changed() {
        if let Ok(mut text) = score_text.single_mut() {
            text.0 = format!("Score: {}", score.0);
        }
    }
    if timer.is_changed() {
        if let Ok(mut text) = time_text.single_mut() {
            text.0 = if timer.running {
                format!("Time: {}", timer.remaining.ceil() as u32)
            } else {
                "TIME UP! (R)".to_string()
            };
        }
    }
    if combo.is_changed() {
        if let Ok(mut text) = combo_text.single_mut() {
            text.0 = if combo.streak >= 2 {
                format!("COMBO x{}", combo.streak.min(MAX_MULTIPLIER))
            } else {
                String::new()
            };
        }
    }
}

/// Let the combo lapse if no hit lands within `COMBO_DECAY` seconds.
fn combo_decay(time: Res<Time>, mut combo: ResMut<Combo>) {
    if combo.streak == 0 {
        return;
    }
    combo.decay -= time.delta_secs();
    if combo.decay <= 0.0 {
        combo.streak = 0;
        combo.decay = 0.0;
    }
}

/// Flash the crosshair yellow and enlarge it briefly after a successful hit.
fn crosshair_feedback(
    time: Res<Time>,
    mut flash: ResMut<HitFlash>,
    mut crosshair: Query<(&mut TextColor, &mut TextFont), With<CrosshairText>>,
) {
    if flash.t > 0.0 {
        flash.t -= time.delta_secs();
    }
    let active = flash.t > 0.0;
    if let Ok((mut color, mut font)) = crosshair.single_mut() {
        if active {
            color.0 = Color::srgb(1.0, 0.85, 0.1);
            font.font_size = 42.0;
        } else {
            color.0 = Color::WHITE;
            font.font_size = 28.0;
        }
    }
}

/// R restarts the round: reset score, combo, and the countdown.
fn restart_round(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut score: ResMut<Score>,
    mut timer: ResMut<GameTimer>,
    mut combo: ResMut<Combo>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) {
        score.0 = 0;
        combo.streak = 0;
        combo.decay = 0.0;
        timer.remaining = ROUND_SECONDS;
        timer.running = true;
        info!("Round restarted.");
    }
}
