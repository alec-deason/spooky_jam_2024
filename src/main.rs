use bevy::{
    prelude::*,
    log::{Level, LogPlugin},
    window::PrimaryWindow,
};
use blenvy::*;
use bevy_mod_picking::prelude::*;


mod build_phase;
mod decay_phase;
mod scoring_phase;
mod block;
mod environmental_decoration;

pub const CAMERA_SCALE: f32 = 0.05;
const SNAP_DISTANCE: f32 = 25.0;
include!(concat!(env!("OUT_DIR"), "/consts.rs"));

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    BuildPhase,
    DecayPhase,
    ScoringPhase,
}


#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SpawnedFrom(Entity);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Spawned(Entity);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Spawner;

#[derive(Component)]
pub struct SavedPosition(Transform);


#[derive(Default, Resource)]
pub struct MousePos(Vec2);

#[derive(Copy, Clone, Component)]
struct Lift<T>(std::marker::PhantomData<T>);
impl <T> Default for Lift<T> {
    fn default() -> Self {
        Lift(Default::default())
    }
}


fn main() {
    App::new()
        .register_type::<SpawnedFrom>()
        .register_type::<Spawner>()

        .init_resource::<MousePos>()

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
        .add_plugins(bevy_hanabi::HanabiPlugin)
        .insert_resource(DebugPickingMode::Normal)
        .add_plugins(crate::block::BlockPlugin)
        .add_plugins(crate::environmental_decoration::EnvironmentalDecorationPlugin)
        .add_plugins(build_phase::BuildPhasePlugin)
        .add_plugins(decay_phase::DecayPhasePlugin)
        .add_plugins(scoring_phase::ScoringPhasePlugin)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1000.,
        })
        .add_systems(Update, update_mouse_pos)
        .run();
}

fn update_mouse_pos(mut mouse_pos: ResMut<MousePos>, q_windows: Query<&Window, With<PrimaryWindow>>) {
    let window = q_windows.single();
    if let Some(position) = window.cursor_position() {
        mouse_pos.0.x = (position.x - window.width() / 2.0) * crate::CAMERA_SCALE;
        mouse_pos.0.y = -(position.y - window.height() / 2.0) * crate::CAMERA_SCALE;
    } else {
        mouse_pos.0 = Vec2::new(0.0, 0.0);
    }
}


pub fn lift_component<T: Component + Clone>(
    mut commands: Commands,
    query: Query<(Entity, &T), Without<BlueprintInfo>>,
    main_blueprint: Query<Entity, With<BlueprintInfo>>,
    parents: Query<&Parent>,
) {
    for (src_entity, component) in &query {
        commands.entity(src_entity).remove::<T>();
        for ancestor in parents.iter_ancestors(src_entity) {
            if main_blueprint.contains(ancestor) {
                commands.entity(ancestor).insert(component.clone());
            }
        }
    }
}
