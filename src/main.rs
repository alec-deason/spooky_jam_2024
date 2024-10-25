use serde_json::Value;

use bevy::{
    asset::AssetMetaCheck,
    gltf::GltfExtras,
    log::{Level, LogPlugin},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_kira_audio::prelude::*;
use bevy_mod_picking::prelude::*;
use blenvy::*;

mod block;
mod block_pool;
mod build_phase;
mod decay_phase;
mod environmental_decoration;
mod music;
mod scoring_phase;

const SNAP_DISTANCE: f32 = 30.0;
include!(concat!(env!("OUT_DIR"), "/consts.rs"));

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    Loading,
    BuildPhase,
    DecayPhase,
    ScoringPhase,
}

#[derive(Resource)]
struct CameraScale(f32);

#[derive(Component)]
struct LoadingScreen;

#[derive(Component)]
struct ExtrasProcessed;
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

#[derive(Resource)]
pub struct PaperTexture(Handle<Image>);

#[derive(Copy, Clone, Component)]
struct Lift<T>(std::marker::PhantomData<T>);
impl<T> Default for Lift<T> {
    fn default() -> Self {
        Lift(Default::default())
    }
}

fn main() {
    App::new()
        .register_type::<SpawnedFrom>()
        .register_type::<Spawner>()
        .init_resource::<MousePos>()
        .insert_resource(CameraScale(1.0))
        .add_plugins(
            DefaultPlugins
                .set(low_latency_window_plugin())
                .set(LogPlugin {
                    level: Level::ERROR,
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .add_plugins(AudioPlugin)
        //.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .insert_state(GameState::Loading)
        .add_plugins(BlenvyPlugin::default())
        .add_plugins(
            DefaultPickingPlugins
                .build()
                .disable::<DebugPickingPlugin>(),
        )
        .insert_resource(DebugPickingMode::Normal)
        .add_plugins(crate::block::BlockPlugin)
        .add_plugins(block_pool::BlockPoolPlugin)
        .add_plugins(music::AudioPlugin)
        .add_plugins(crate::environmental_decoration::EnvironmentalDecorationPlugin)
        .add_plugins(build_phase::BuildPhasePlugin)
        .add_plugins(decay_phase::DecayPhasePlugin)
        .add_plugins(scoring_phase::ScoringPhasePlugin)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1000.,
        })
        .add_systems(
            Update,
            (
                maintain_camera_scale,
                update_mouse_pos,
                check_for_gltf_extras,
                fix_materials,
            ),
        )
        .add_systems(
            PostUpdate,
            check_loading_completion.run_if(in_state(GameState::Loading)),
        )
        .add_systems(Startup, (blank_screen, start_load))
        .run();
}

fn start_load(mut commands: Commands, assets: ResMut<AssetServer>) {
    commands.insert_resource(PaperTexture(
        assets.load("indieground-vintagepaper-textures-03.jpg"),
    ));
}

fn fix_materials(
    mut materials: ResMut<Assets<StandardMaterial>>,
    paper_texture: Res<PaperTexture>,
) {
    if materials.is_changed() {
        for (_, material) in materials.iter_mut() {
            if material.base_color_texture.is_none() {
                material.emissive_texture = Some(paper_texture.0.clone());
            }
        }
    }
}

fn blank_screen(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn(PbrBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 9.9)),
            mesh: meshes.add(Rectangle {
                half_size: Vec2::new(100.0, 100.0),
            }),
            material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.0, 0.0, 0.0, 1.0),
                ..default()
            }),
            ..default()
        })
        .insert(LoadingScreen);
}

fn maintain_camera_scale(projection: Query<&Projection>, mut camera_scale: ResMut<CameraScale>) {
    for projection in &projection {
        if let Projection::Orthographic(projection) = projection {
            camera_scale.0 = projection.scale;
        }
    }
}

fn check_loading_completion(
    mut commands: Commands,
    query: Query<
        Entity,
        (
            With<BlueprintInfo>,
            Or<(
                Without<BlueprintInstanceReady>,
                With<block_pool::TempBlockPoolResident>,
                With<block_pool::TempDecayedPoolResident>,
            )>,
        ),
    >,
    loading_screen: Query<Entity, With<LoadingScreen>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if query.is_empty() {
        let entity = loading_screen.single();
        commands.entity(entity).despawn_recursive();
        next_state.set(GameState::BuildPhase);
    }
}

fn update_mouse_pos(
    mut mouse_pos: ResMut<MousePos>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    camera_scale: Res<CameraScale>,
) {
    let window = q_windows.single();
    if let Some(position) = window.cursor_position() {
        mouse_pos.0.x = (position.x - window.width() / 2.0) * camera_scale.0;
        mouse_pos.0.y = -(position.y - window.height() / 2.0) * camera_scale.0;
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

fn check_for_gltf_extras(
    mut commands: Commands,
    gltf_extras_per_entity: Query<(Entity, &GltfExtras), Without<ExtrasProcessed>>,
    children: Query<&Children>,
    parents: Query<&Parent>,
    ready: Query<Entity, With<BlueprintInstanceReady>>,
    mut material_handle: Query<&mut Handle<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, extras) in gltf_extras_per_entity.iter() {
        let v: Value = serde_json::from_str(&extras.value).unwrap();
        if let Some(color) = v.get("color") {
            if ready.contains(entity) || parents.iter_ancestors(entity).any(|e| ready.contains(e)) {
                let r = color.get(0).unwrap().as_f64().unwrap() as f32;
                let g = color.get(1).unwrap().as_f64().unwrap() as f32;
                let b = color.get(2).unwrap().as_f64().unwrap() as f32;
                for child_entity in children.iter_descendants(entity) {
                    if let Ok(mut handle) = material_handle.get_mut(child_entity) {
                        if let Some(material) = materials.get_mut(&*handle) {
                            let mut new_material = material.clone();
                            new_material.emissive = LinearRgba::new(r, g, b, 1.0);
                            *handle = materials.add(new_material);
                            commands.entity(entity).insert(ExtrasProcessed);
                        }
                    }
                }
            }
        }
    }
}
