use bevy::{
    prelude::*,
    log::{Level, LogPlugin},
};
use blenvy::*;
use bevy_mod_picking::prelude::*;


mod build_phase;
mod decay_phase;
mod scoring_phase;
mod block;

pub const CAMERA_SCALE: f32 = 0.05;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    BuildPhase,
    DecayPhase,
    ScoreingPhase,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct FoundationIdle;


fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(low_latency_window_plugin()).set(LogPlugin {
            level: Level::ERROR,
            ..default()
        }))
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
        .register_type::<FoundationIdle>()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1000.,
        })
        .add_systems(Update, water_animation_control)
        .run();
}

pub fn water_animation_control(
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
