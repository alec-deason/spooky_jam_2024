use bevy::{
    prelude::*,
    log::{Level, LogPlugin},
};
use blenvy::*;
use bevy_mod_picking::prelude::*;
use bevy_tweening::*;


mod build_phase;
mod decay_phase;
mod scoring_phase;
mod block;

pub const CAMERA_SCALE: f32 = 0.05;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    BuildPhase,
    DecayPhase,
    ScoringPhase,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct FoundationIdle;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Star;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub enum Sky {
    Color(Color, f32),
    Transition {
        start_color: Color,
        end_color: Color,
        start_time: std::time::Duration,
        end_time: std::time::Duration,
        start_star_brightness: f32,
        end_star_brightness: f32,
    }
}

impl Sky {
    pub fn to_night(&mut self, now: std::time::Duration) {
        let color = self.current_color(now);
        *self = Sky::Transition {
            start_color: color,
            end_color: Color::srgba(0.002, 0.001, 0.012, 1.0),
            start_time: now,
            end_time: now + std::time::Duration::from_secs(2),
            start_star_brightness: self.current_star_brightness(now),
            end_star_brightness: 1.0,
        };
    }

    pub fn current_color(&self, now: std::time::Duration) -> Color {
        match self {
            Sky::Color(color, _) => {
                *color
            }
            Sky::Transition { start_color, end_color, start_time, end_time, .. } => {
                let t = now - *start_time;
                let f = t.as_secs_f32() / (*end_time - *start_time).as_secs_f32();
                start_color.mix(end_color, f)
            }
        }
    }

    pub fn current_star_brightness(&self, now: std::time::Duration) -> f32 {
        match self {
            Sky::Color(_color, star_brightness) => {
                *star_brightness
            }
            Sky::Transition { start_star_brightness, end_star_brightness, start_time, end_time, .. } => {
                let t = now - *start_time;
                let f = t.as_secs_f32() / (*end_time - *start_time).as_secs_f32();
                start_star_brightness + (end_star_brightness - start_star_brightness) * f
            }
        }
    }
}


fn main() {
    App::new()
        .register_type::<FoundationIdle>()
        .register_type::<Sky>()
        .register_type::<Star>()

        .add_plugins(DefaultPlugins.set(low_latency_window_plugin()).set(LogPlugin {
            level: Level::ERROR,
            ..default()
        }))
        //.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .insert_state(GameState::BuildPhase)
        .add_plugins(BlenvyPlugin::default())
        .add_plugins(DefaultPickingPlugins
            .build()
            .disable::<DebugPickingPlugin>())
        .insert_resource(DebugPickingMode::Normal)
        .add_plugins(crate::block::BlockPlugin)
        .add_plugins(build_phase::BuildPhasePlugin)
        .add_plugins(decay_phase::DecayPhasePlugin)
        .add_plugins(scoring_phase::ScoringPhasePlugin)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1000.,
        })
        .add_systems(Update, (water_animation_control, star_animation, sky_color_animation))
        .run();
}

fn water_animation_control(
    animations: Query<(&BlueprintAnimationPlayerLink, &BlueprintAnimations), With<FoundationIdle>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (link, animations) in animations.iter() {
        let (mut animation_player, mut transition) =
            animation_players.get_mut(link.0).unwrap();
        if let Some(animation) = animations.named_indices.get("Idle") {
            if !animation_player.is_playing_animation(*animation) {
                transition
                    .play(&mut animation_player, *animation, std::time::Duration::ZERO)
                    .repeat();
            }
        }
    }
}

fn sky_color_animation(
    mut query: Query<(&mut Sky, &Handle<StandardMaterial>)>,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut sky_state, material_handle) in &mut query {
        if let Some(material) = materials.get_mut(material_handle) {
            match &*sky_state {
                Sky::Color(color, _) => material.emissive= (*color).into(),
                Sky::Transition { end_color, end_time, end_star_brightness, .. } => {
                    let now = time.elapsed();
                    if *end_time <= now {
                        *sky_state = Sky::Color(*end_color, *end_star_brightness);
                    }
                    material.emissive = sky_state.current_color(now).into();
                }
            }
        }
    }
}

fn star_animation(
    mut query: Query<(&mut Sky, &Handle<StandardMaterial>), With<Star>>,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed();
    for (mut sky_state, material_handle) in &mut query {
        if let Some(material) = materials.get_mut(material_handle) {
            let b = match &*sky_state {
                Sky::Color(_color, star_brightness) => *star_brightness,
                Sky::Transition { end_color, end_time, end_star_brightness, .. } => {
                    if *end_time <= now {
                        *sky_state = Sky::Color(*end_color, *end_star_brightness);
                    }
                    sky_state.current_star_brightness(now)
                }
            };
            let c = sky_state.current_color(now).mix(&Color::WHITE, b);
            println!("{c:?}");
            material.emissive = c.into();
        }
    }
}
