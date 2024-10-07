use std::path::PathBuf;
use std::f32::consts::PI;

use bevy::{
    prelude::*,
    color::palettes::css::*,
    pbr::CascadeShadowConfigBuilder,
    input::{
        keyboard::{KeyboardInput, Key},
    },
    window::PrimaryWindow,
};
use blenvy::{
    BlueprintAnimationPlayerLink, BlueprintAnimations, BlueprintInfo, GameWorldTag,
    HideUntilReady, SpawnBlueprint,
};
use bevy_mod_picking::prelude::*;

const CAMERA_SCALE: f32 = 0.05;
const SNAP_DISTANCE: f32 = 15.0;
include!(concat!(env!("OUT_DIR"), "/consts.rs"));

#[derive(Component)]
pub struct SavedPosition(Transform);

#[derive(Copy, Clone, Component)]
pub struct AwaitingPlacement;

#[derive(Component)]
pub struct Snapped;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct BlockSpawner;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SpawnedFrom(Entity);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SpawnedBlock(Entity);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct FoundationIdle;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct BlockTenticle;

#[derive(Default, Resource)]
pub struct MousePos(Vec2);

pub struct BuildPhasePlugin;

impl Plugin for BuildPhasePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(crate::block::BlockPlugin)
            .insert_resource(AmbientLight {
                color: Color::WHITE,
                brightness: 1000.,
            })
            .register_type::<BlockSpawner>()
            .register_type::<FoundationIdle>()
            .register_type::<BlockTenticle>()
            .init_resource::<MousePos>()
            .init_resource::<Events<Pointer<Click>>>()
            .add_systems(Update, (spawn_block, follow_mouse, update_mouse_pos, place_block, save_position, stop_drag))
            .add_systems(Update, (water_animation_control, tenticle_animation_control))
            .add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, asset_server: Res<AssetServer>, mut materials: ResMut<Assets<StandardMaterial>>,) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 10.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            projection: OrthographicProjection {
                scale: CAMERA_SCALE,
                ..default()
            }.into(),
            ..default()
        },
    ));
    /*
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(20.0, 0.0, 10.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), -Vec3::Y),
        ..default()
    });
    */
    commands.spawn((
        BlueprintInfo::from_path("levels/_foundation.glb"),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        SpawnBlueprint,
        HideUntilReady,
        GameWorldTag,
    ));
}

fn update_mouse_pos(mut mouse_pos: ResMut<MousePos>, q_windows: Query<&Window, With<PrimaryWindow>>) {
    let window = q_windows.single();
    if let Some(position) = window.cursor_position() {
        mouse_pos.0.x = (position.x - window.width() / 2.0) * CAMERA_SCALE;
        mouse_pos.0.y = -(position.y - window.height() / 2.0) * CAMERA_SCALE;
    } else {
        mouse_pos.0 = Vec2::new(0.0, 0.0);
    }
}

fn spawn_block(mut commands: Commands, spawn_points: Query<(Entity, &GlobalTransform), (With<BlockSpawner>, Without<SpawnedBlock>)>) {
    if let Some((spawner_entity, spawner_transform)) = spawn_points.iter().next() {
        let path = &BLOCKS[fastrand::usize(0..BLOCKS.len())];
        let mut transform = Transform::from(spawner_transform.clone()).with_scale(Vec3::ONE);
        if path.contains("reversable") && fastrand::f32() > 0.5 {
            transform = transform.with_scale(Vec3::new(-1.0, 1.0, 1.0));
        };
        let block_entity = commands.spawn((
            BlueprintInfo::from_path(path),
            transform,
            SpawnBlueprint,
            HideUntilReady,
            GameWorldTag,
            SpawnedFrom(spawner_entity),
            PickableBundle::default(),
            On::<Pointer<DragStart>>::listener_insert(AwaitingPlacement),
            On::<Pointer<DragEnd>>::listener_remove::<AwaitingPlacement>(),
        )).id();
        commands.entity(spawner_entity).insert(SpawnedBlock(block_entity));
    }
}

fn stop_drag(mut commands: Commands, mut query: Query<(Entity, &SpawnedFrom, &mut Transform, Option<&Snapped>, Option<&SavedPosition>), (Without<AwaitingPlacement>, Or<(With<Snapped>, With<SavedPosition>)>)>, children_query: Query<&Children>) {
    for (entity, spawned_from, mut transform, maybe_snapped, maybe_saved) in &mut query {
        commands.entity(entity).remove::<Snapped>().remove::<SavedPosition>();
        if maybe_snapped.is_none() {
            if let Some(saved) = maybe_saved {
                *transform = saved.0.clone();
            }
        } else {
            commands.entity(spawned_from.0).remove::<SpawnedBlock>();
            commands.entity(entity).insert(Pickable::IGNORE);
            for descendant in children_query.iter_descendants(entity) {
                commands.entity(descendant).insert(Pickable::IGNORE);
            }
        }
    }
}

fn save_position(mut commands: Commands, mut query: Query<(Entity, &Transform), (With<AwaitingPlacement>, Without<SavedPosition>)>) {
    for (entity, transform) in &mut query {
        commands.entity(entity).insert(SavedPosition(transform.clone()));
    }
}

fn place_block(mut commands: Commands, mouse_button_input: Res<ButtonInput<MouseButton>>, query: Query<Entity, (With<AwaitingPlacement>, With<Snapped>)>) {
    if mouse_button_input.just_released(MouseButton::Left) {
        for entity in &query {
            commands.entity(entity)
                .remove::<AwaitingPlacement>();
        }
    }
}

fn follow_mouse(mut commands: Commands, mut query: Query<(Entity, &mut Transform, &crate::block::Anchors), With<AwaitingPlacement>>, mouse_pos: Res<MousePos>, others: Query<(&Transform, &crate::block::Anchors), Without<AwaitingPlacement>>) {
    for (entity, mut transform, anchors) in &mut query {
        let mut snapped = false;

        transform.rotation = Quat::IDENTITY;
        let mut maybe_pos = transform.translation.clone();
        maybe_pos.x = mouse_pos.0.x;
        maybe_pos.y = mouse_pos.0.y;

        'outer: for (other_transform, other_anchors) in &others {
            for anchor in &anchors.0 {
                for other_anchor in &other_anchors.0 {
                    let d = (maybe_pos + *anchor)-(other_transform.translation + *other_anchor);
                    if d.length() < SNAP_DISTANCE*CAMERA_SCALE {
                        maybe_pos.x -= d.x;
                        maybe_pos.y -= d.y;
                        snapped = true;
                        break 'outer;
                    }
                }
            }
        }

        transform.translation.x = maybe_pos.x;
        transform.translation.y = maybe_pos.y;

        if snapped {
            commands.entity(entity).insert(Snapped);
        } else {
            commands.entity(entity).remove::<Snapped>();
        }
    }
}

pub fn water_animation_control(
    animations: Query<(&BlueprintAnimationPlayerLink, &BlueprintAnimations), With<FoundationIdle>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    mut done: Local<bool>
) {
    if !*done {
        for (link, animations) in animations.iter() {
            println!("BLAHHH");
            let (mut animation_player, mut transition) =
                animation_players.get_mut(link.0).unwrap();
            if let Some(animation) = animations.named_indices.get("Idle") {
                transition
                    .play(&mut animation_player, *animation, std::time::Duration::ZERO)
                    .repeat();
                *done = true;
            }
        }
    }
}

pub fn tenticle_animation_control(
    animations: Query<(&BlueprintAnimationPlayerLink, &BlueprintAnimations), With<BlockTenticle>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    mut done: Local<bool>
) {
    if !*done {
        for (link, animations) in animations.iter() {
            println!("BLAHHH");
            let (mut animation_player, mut transition) =
                animation_players.get_mut(link.0).unwrap();
            if let Some(animation) = animations.named_indices.get("Extend") {
                transition
                    .play(&mut animation_player, *animation, std::time::Duration::ZERO)
                    .repeat();
                *done = true;
            }
        }
    }
}
