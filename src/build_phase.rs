use std::path::PathBuf;
use std::f32::consts::PI;

use bevy::{
    prelude::*,
    color::palettes::css::*,
    pbr::CascadeShadowConfigBuilder,
    animation::AnimationTarget,
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
const SNAP_DISTANCE: f32 = 25.0;

include!(concat!(env!("OUT_DIR"), "/consts.rs"));

#[derive(Component)]
pub struct SavedPosition(Transform);

#[derive(Copy, Clone, Component)]
pub struct AwaitingPlacement;

#[derive(Debug, Component)]
pub struct Snapped {
    a_entity: Entity,
    a_anchor: usize,
    b_entity: Entity,
    b_anchor: usize,
}

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

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Water;

#[derive(Default, Resource)]
pub struct MousePos(Vec2);

#[derive(Default, Component)]
pub struct Retracting;

#[derive(Default, Component, Reflect)]
#[reflect(Component)]
pub struct Extending;

#[derive(Default, Component, Reflect)]
#[reflect(Component)]
pub struct Idle;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct NeedsClearance;


#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TentacleSpawner(Entity);

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
            .register_type::<SpawnedFrom>()
            .register_type::<Extending>()
            .register_type::<Water>()
            .init_resource::<MousePos>()
            .init_resource::<Events<Pointer<Click>>>()
            .add_systems(Update, (spawn_block, follow_mouse, update_mouse_pos, save_position, stop_drag, update_tentacle_spawners, clear_blocked_anchors))
            .add_systems(PostUpdate, blocks_track_spawners)
            .add_systems(Update, (water_animation_control, start_retract, tentacle_retracting, tentacle_extending, tentacle_idle))
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

fn start_retract(
    mut commands: Commands,
    tentacles: Query<(Entity, &TentacleSpawner), With<Idle>>,
    spawn_points: Query<Entity, (With<BlockSpawner>, Without<SpawnedBlock>)>,
) {
    for (entity, tentacle_spawner) in &tentacles {
        if spawn_points.contains(tentacle_spawner.0) {
            commands.entity(entity).remove::<Idle>().insert(Retracting);
        }
    }
}

fn spawn_block(
    mut commands: Commands,
    tentacles: Query<&TentacleSpawner, With<Extending>>,
    spawn_points: Query<Entity, (With<BlockSpawner>, Without<SpawnedBlock>)>,
) {
    for tentacle_spawner in &tentacles {
        if let Ok(spawner_entity) = spawn_points.get(tentacle_spawner.0) {
            let path = &BLOCKS[fastrand::usize(0..BLOCKS.len())];
            let mut transform = Transform::default();
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
}

fn stop_drag(
    mut commands: Commands,
    mut query: Query<(Entity, &SpawnedFrom, &mut Transform, Option<&mut Snapped>, Option<&SavedPosition>), (Without<AwaitingPlacement>, Or<(With<Snapped>, With<SavedPosition>)>)>,
    children_query: Query<&Children>,
    water: Query<&GlobalTransform, With<Water>>,
    mut anchors: Query<&mut crate::block::Anchors>,
) {
    for (entity, spawned_from, mut transform, maybe_snapped, maybe_saved) in &mut query {
        commands.entity(entity).remove::<Snapped>().remove::<SavedPosition>();
        if let Some(Snapped { a_entity, a_anchor, b_entity, b_anchor }) = maybe_snapped.map(|v| v.into_inner()) {
            if let Ok(mut anchors) = anchors.get_mut(*a_entity) {
                anchors.0[*a_anchor].2 = Some(*b_entity);
            }
            if let Ok(mut anchors) = anchors.get_mut(*b_entity) {
                anchors.0[*b_anchor].2 = Some(*a_entity);
            }
            commands.entity(spawned_from.0).remove::<SpawnedBlock>();
            commands.entity(entity).insert(Pickable::IGNORE).insert(NeedsClearance);
            for descendant in children_query.iter_descendants(entity) {
                commands.entity(descendant).insert(Pickable::IGNORE);
            }
        } else {
            for water_transform in &water {
                if transform.translation.y < water_transform.translation().y {
                    commands.entity(entity).despawn_recursive();
                    commands.entity(spawned_from.0).remove::<SpawnedBlock>().insert(Retracting);
                    break
                }
            }
            if let Some(saved) = maybe_saved {
                *transform = saved.0.clone();
            }
        }
    }
}

fn save_position(mut commands: Commands, mut query: Query<(Entity, &Transform), (With<AwaitingPlacement>, Without<SavedPosition>)>) {
    for (entity, transform) in &mut query {
        commands.entity(entity).insert(SavedPosition(transform.clone()));
    }
}

fn follow_mouse(mut commands: Commands, mut query: Query<(Entity, &mut Transform, &crate::block::Anchors), With<AwaitingPlacement>>, mouse_pos: Res<MousePos>, others: Query<(Entity, &Transform, &crate::block::Anchors), Without<AwaitingPlacement>>) {
    for (entity, mut transform, anchors) in &mut query {
        let mut snapped = None;

        transform.rotation = Quat::IDENTITY;
        let mut maybe_pos = transform.translation.clone();
        maybe_pos.x = mouse_pos.0.x;
        maybe_pos.y = mouse_pos.0.y;

        'outer: for (other_entity, other_transform, other_anchors) in &others {
            for (a_anchor, (anchor, color, bound_entity)) in anchors.0.iter().enumerate() {
                if bound_entity.is_some() {
                    continue
                }
                for (b_anchor, (other_anchor, other_color, other_bound_entity)) in other_anchors.0.iter().enumerate() {
                    if other_bound_entity.is_some() || !color.compatible(*other_color) {
                        continue
                    }
                    let d = (maybe_pos + *anchor)-(other_transform.translation + *other_anchor);
                    if d.length() < SNAP_DISTANCE*CAMERA_SCALE {
                        maybe_pos.x -= d.x;
                        maybe_pos.y -= d.y;
                        snapped = Some(Snapped {
                            a_entity: entity,
                            a_anchor,
                            b_entity: other_entity,
                            b_anchor,
                        });
                        break 'outer;
                    }
                }
            }
        }

        transform.translation.x = maybe_pos.x;
        transform.translation.y = maybe_pos.y;

        if let Some(snapped) = snapped {
            commands.entity(entity).insert(snapped);
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

pub fn tentacle_idle(
    animations: Query<(Entity, &BlueprintAnimationPlayerLink, &BlueprintAnimations), With<Idle>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (entity, link, animations) in animations.iter() {
        let (mut animation_player, mut transition) =
            animation_players.get_mut(link.0).unwrap();
        if let Some(idle_animation) = animations.named_indices.get("Extended Idle") {
            if !animation_player.is_playing_animation(*idle_animation) {
                transition
                    .play(&mut animation_player, *idle_animation, std::time::Duration::from_millis(250))
                    .repeat()
                    .set_speed((fastrand::f32()*0.1 + 0.95) * if fastrand::f32() > 0.5 { 1.0 } else { -1.0 })
                    .seek_to(fastrand::f32()*4.0);
            }
        }
    }
}

pub fn tentacle_extending(
    mut commands: Commands,
    animations: Query<(Entity, &BlueprintAnimationPlayerLink, &BlueprintAnimations), With<Extending>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (entity, link, animations) in animations.iter() {
        let (mut animation_player, mut transition) =
            animation_players.get_mut(link.0).unwrap();
        if let Some(animation) = animations.named_indices.get("Extend") {
            if animation_player.is_playing_animation(*animation) {
                if animation_player.all_finished() {
                    commands.entity(entity).remove::<Extending>().insert(Idle);
                }
            } else {
                transition
                    .play(&mut animation_player, *animation, std::time::Duration::ZERO);
            }
        }
    }
}

pub fn tentacle_retracting(
    mut commands: Commands,
    animations: Query<(Entity, &BlueprintAnimationPlayerLink, &BlueprintAnimations), With<Retracting>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (entity, link, animations) in animations.iter() {
        let (mut animation_player, mut transition) =
            animation_players.get_mut(link.0).unwrap();
        if let Some(animation) = animations.named_indices.get("Retract") {
            if animation_player.is_playing_animation(*animation) {
                if animation_player.all_finished() {
                    commands.entity(entity).remove::<Retracting>().insert(Extending);
                }
            } else {
                transition
                    .play(&mut animation_player, *animation, std::time::Duration::ZERO);
            }
        }
    }
}

fn blocks_track_spawners(spawners: Query<(&GlobalTransform, &SpawnedBlock)>, mut blocks: Query<&mut Transform, Without<AwaitingPlacement>>) {
    for (spawner_transform, spawned_block) in &spawners {
        if let Ok(mut block_transform) = blocks.get_mut(spawned_block.0) {
            let (_scale, rotation, translation) = spawner_transform.to_scale_rotation_translation();
            block_transform.rotation = rotation;
            block_transform.translation = translation;
        }
    }
}

fn update_tentacle_spawners(mut commands: Commands, query: Query<Entity, (With<BlockTenticle>, Without<TentacleSpawner>)>, children: Query<&Children>, spawners: Query<Entity, With<BlockSpawner>>) {
    for tentacle_entity in &query {
        for descendant in children.iter_descendants(tentacle_entity) {
            if let Ok(spawner_entity) = spawners.get(descendant) {
                commands.entity(tentacle_entity).insert(TentacleSpawner(spawner_entity));
                break;
            }
        }
    }
}

fn clear_blocked_anchors(mut commands: Commands, mut newly_placed: Query<(Entity, &GlobalTransform, &mut crate::block::Anchors), With<NeedsClearance>>, mut others: Query<(&GlobalTransform, &mut crate::block::Anchors), Without<NeedsClearance>>) {
    for (entity, transform, mut anchors) in &mut newly_placed {
        commands.entity(entity).remove::<NeedsClearance>();
        println!("POOP");
        for (other_transform, mut other_anchors) in &mut others {
            anchors.0.retain(|(anchor, _color, bound_entity)| {
                if bound_entity.is_some() {
                    return true;
                }
                let mut retain = true;
                other_anchors.0.retain(|(other_anchor, _other_color, other_bound_entity)| {
                    if other_bound_entity.is_some() {
                        return true;
                    }
                    let d = (transform.translation() + *anchor)-(other_transform.translation() + *other_anchor);
                    if d.length() < SNAP_DISTANCE*CAMERA_SCALE {
                        retain = false;
                        false
                    } else {
                        true
                    }
                });
                retain
            });
        }
    }
}
