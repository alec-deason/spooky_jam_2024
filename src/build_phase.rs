use bevy::{
    prelude::*,
};
use blenvy::{
    BlueprintAnimationPlayerLink, BlueprintAnimations, BlueprintInfo, GameWorldTag,
    HideUntilReady, SpawnBlueprint,
};
use bevy_mod_picking::prelude::*;

use crate::{SNAP_DISTANCE, CAMERA_SCALE, GameState, SavedPosition, Spawned, SpawnedFrom, Spawner, environmental_decoration::{Water, Sky}, MousePos, BLOCKS, block::{DecayedRepresentation, Block, AnchorState}, Lift};

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum PhasePhase {
    Running,
    ShuttingDown,
    Idle,
}


#[derive(Copy, Clone, Component)]
pub struct AwaitingPlacement;

#[derive(Copy, Clone, Component)]
pub struct OnTentacle;

#[derive(Debug, Component)]
pub struct Snapped {
    a_entity: Entity,
    a_anchor: usize,
    b_entity: Entity,
    b_anchor: usize,
    a_translation: Vec3,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Foundation;


#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Tentacle;

#[derive(Default, Component)]
pub struct Retracting;

#[derive(Default, Component, Reflect)]
#[reflect(Component)]
pub struct Extending;

#[derive(Default, Component, Reflect)]
#[reflect(Component)]
pub struct Dead;

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
            .insert_state(PhasePhase::Running)
            .register_type::<Tentacle>()
            .register_type::<Extending>()
            .register_type::<Foundation>()
            .init_resource::<Events<Pointer<Click>>>()

            .add_systems(Update, (start_retract, tentacle_retracting, tentacle_extending, tentacle_idle).run_if(in_state(crate::GameState::BuildPhase)))

            .add_systems(Update, (
                spawn_block,
                follow_mouse,
                start_drag,
                stop_drag,
                update_tentacle_spawners,
            ).run_if(in_state(crate::GameState::BuildPhase)).run_if(in_state(PhasePhase::Running)))
            .add_systems(PostUpdate, (check_completion.run_if(in_state(PhasePhase::Running)), blocks_track_spawners).run_if(in_state(crate::GameState::BuildPhase)))
            .add_systems(OnEnter(crate::GameState::BuildPhase), setup)
            .add_systems(OnEnter(crate::GameState::BuildPhase), |mut next_state: ResMut<NextState<PhasePhase>>| { next_state.set(PhasePhase::Running) })

            .add_systems(OnEnter(PhasePhase::ShuttingDown), retract_tentacles)
            .add_systems(Update, (retract_tentacles, check_shutdown_completion).run_if(in_state(PhasePhase::ShuttingDown)))
            .add_systems(OnExit(crate::GameState::BuildPhase), (despawn_tentacles, despawn_spare_blocks))
            ;
    }
}

fn setup(mut commands: Commands) {
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
    commands.spawn((
        BlueprintInfo::from_path("levels/_foundation.glb"),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        SpawnBlueprint,
        HideUntilReady,
        GameWorldTag,
    ));
}


fn start_retract(
    mut commands: Commands,
    tentacles: Query<(Entity, &TentacleSpawner), With<Idle>>,
    spawn_points: Query<Entity, (With<Spawner>, Without<Spawned>)>,
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
    spawn_points: Query<Entity, (With<Spawner>, Without<Spawned>)>,
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
                OnTentacle,
                Block,
                SpawnedFrom(spawner_entity),
                Lift::<DecayedRepresentation>::default(),
                PickableBundle::default(),
                On::<Pointer<DragStart>>::listener_insert(AwaitingPlacement),
                On::<Pointer<DragEnd>>::listener_remove::<AwaitingPlacement>(),
            )).id();
            commands.entity(spawner_entity).insert(Spawned(block_entity));
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
        if let Some(Snapped { a_entity, a_anchor, b_entity, b_anchor, a_translation }) = maybe_snapped.map(|v| v.into_inner()) {
            if let Ok(mut anchors) = anchors.get_mut(*a_entity) {
                anchors.0[*a_anchor].2 = AnchorState::Occupied(*b_entity);
            }
            if let Ok(mut anchors) = anchors.get_mut(*b_entity) {
                anchors.0[*b_anchor].2 = AnchorState::Occupied(*a_entity);
            }
            commands.entity(spawned_from.0).remove::<Spawned>();
            commands.entity(entity).remove::<OnTentacle>().insert(Pickable::IGNORE).insert(NeedsClearance);
            for descendant in children_query.iter_descendants(entity) {
                commands.entity(descendant).insert(Pickable::IGNORE);
            }
            transform.translation = *a_translation;
        } else {
            for water_transform in &water {
                if transform.translation.y < water_transform.translation().y {
                    commands.entity(entity).despawn_recursive();
                    commands.entity(spawned_from.0).remove::<Spawned>().insert(Retracting);
                    break
                }
            }
            if let Some(saved) = maybe_saved {
                *transform = saved.0.clone();
            }
        }
    }
}

fn start_drag(mut commands: Commands, mut query: Query<(Entity, &mut Transform), (With<AwaitingPlacement>, Without<SavedPosition>)>, foundation: Query<&GlobalTransform, With<Foundation>>) {
    if let Some(foundation) = foundation.iter().next() {
        let foundation_z = foundation.translation().z;
        for (entity, mut transform) in &mut query {
            commands.entity(entity).insert(SavedPosition(transform.clone()));
            transform.translation.z = foundation_z;
        }
    }
}

fn follow_mouse(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &crate::block::Anchors, Option<&crate::block::InCollision>), With<AwaitingPlacement>>,
    mouse_pos: Res<MousePos>,
    others: Query<(Entity, &Transform, &crate::block::Anchors), (Without<AwaitingPlacement>, Without<OnTentacle>)>)
        {
    for (entity, mut transform, anchors, in_collision) in &mut query {
        let mut snapped = None;
        let mut snap_dirty = false;
        transform.rotation = Quat::IDENTITY;
        let mut maybe_pos = transform.translation.clone();
        maybe_pos.x = mouse_pos.0.x;
        maybe_pos.y = mouse_pos.0.y;

        let mut min_distance = std::f32::INFINITY;
        if in_collision.is_none() {
            for (other_entity, other_transform, other_anchors) in &others {
                for (a_anchor, (anchor, color, anchor_state)) in anchors.0.iter().enumerate() {
                    if !(matches!(anchor_state, AnchorState::Clear) || matches!(anchor_state, AnchorState::Blocked(e) if *e == other_entity)) {
                        snap_dirty = true;
                        continue
                    }
                    for (b_anchor, (other_anchor, other_color, other_anchor_state)) in other_anchors.0.iter().enumerate() {
                        if !(matches!(other_anchor_state, AnchorState::Clear) || matches!(other_anchor_state, AnchorState::Blocked(e) if *e == entity)) || !color.compatible(*other_color) {
                            snap_dirty = true;
                            continue
                        }
                        let d = (maybe_pos + *anchor)-(other_transform.translation + *other_anchor);
                        let dist = d.length();
                        if dist < SNAP_DISTANCE*CAMERA_SCALE {
                            if dist < min_distance {
                                min_distance = dist;
                                maybe_pos.x = mouse_pos.0.x - d.x;
                                maybe_pos.y = mouse_pos.0.y - d.y;
                                snapped = Some(Snapped {
                                    a_entity: entity,
                                    a_anchor,
                                    b_entity: other_entity,
                                    b_anchor,
                                    a_translation: maybe_pos,
                                });
                            }
                            snap_dirty = true;
                        }
                    }
                }
            }
        }

        transform.translation.x = maybe_pos.x;
        transform.translation.y = maybe_pos.y;

        if snap_dirty {
            if let Some(snapped) = snapped {
                commands.entity(entity).insert(snapped);
            } else {
                commands.entity(entity).remove::<Snapped>();
            }
        }
    }
}


pub fn tentacle_idle(
    animations: Query<(&BlueprintAnimationPlayerLink, &BlueprintAnimations), With<Idle>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (link, animations) in animations.iter() {
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
    animations: Query<(Entity, &BlueprintAnimationPlayerLink, &BlueprintAnimations, Option<&Dead>), With<Retracting>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (entity, link, animations, maybe_dead) in animations.iter() {
        let (mut animation_player, mut transition) =
            animation_players.get_mut(link.0).unwrap();
        if let Some(animation) = animations.named_indices.get("Retract") {
            if animation_player.is_playing_animation(*animation) {
                if animation_player.all_finished() {
                    commands.entity(entity).remove::<Retracting>();
                    if maybe_dead.is_none() {
                        commands.entity(entity).insert(Extending);
                    }
                }
            } else {
                transition
                    .play(&mut animation_player, *animation, std::time::Duration::ZERO);
            }
        }
    }
}

fn blocks_track_spawners(spawners: Query<(&GlobalTransform, &Spawned)>, mut blocks: Query<&mut Transform, Without<AwaitingPlacement>>) {
    for (spawner_transform, spawned_block) in &spawners {
        if let Ok(mut block_transform) = blocks.get_mut(spawned_block.0) {
            let (_scale, rotation, translation) = spawner_transform.to_scale_rotation_translation();
            block_transform.rotation = rotation;
            block_transform.translation = translation;
        }
    }
}

fn update_tentacle_spawners(mut commands: Commands, query: Query<Entity, (With<Tentacle>, Without<TentacleSpawner>)>, children: Query<&Children>, spawners: Query<Entity, With<Spawner>>) {
    for tentacle_entity in &query {
        for descendant in children.iter_descendants(tentacle_entity) {
            if let Ok(spawner_entity) = spawners.get(descendant) {
                commands.entity(tentacle_entity).insert(TentacleSpawner(spawner_entity));
                break;
            }
        }
    }
}

fn check_completion(
    in_drag: Query<Entity, With<AwaitingPlacement>>,
    query: Query<&crate::block::Anchors, Without<OnTentacle>>,
    mut sky: Query<&mut Sky>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<PhasePhase>>,
    mut delay: Local<bevy::time::Stopwatch>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    delay.tick(time.delta());
    if !in_drag.is_empty() {
        return
    }

    let mut done = None;
    'outer: for anchors in &query {
        for (_, color, anchor_state) in &anchors.0 {
            if *color == crate::block::AnchorColor::Up {
                if done.is_none() {
                    done = Some(true);
                }
                if *anchor_state == AnchorState::Clear {
                    done = Some(false);
                    break 'outer;
                }
            }
        }
    }

    if keyboard.pressed(KeyCode::Space) {
        done = Some(true);
    }

    if done.unwrap_or(false) {
        if delay.elapsed() > std::time::Duration::from_millis(250) {
            for mut sky in &mut sky {
                sky.to_night(time.elapsed());
            }
            next_state.set(PhasePhase::ShuttingDown);
        }
    } else {
        delay.reset();
    }
}

fn retract_tentacles(mut commands: Commands, query: Query<Entity, (With<Tentacle>, Without<Extending>, Without<Dead>)>) {
    for entity in &query {
        commands.entity(entity).remove::<Idle>().insert(Retracting).insert(Dead);
    }
}

fn despawn_tentacles(mut commands: Commands, query: Query<Entity, With<Tentacle>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn despawn_spare_blocks(mut commands: Commands, query: Query<Entity, With<OnTentacle>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn check_shutdown_completion(
    mut next_state: ResMut<NextState<GameState>>,
    mut next_local_state: ResMut<NextState<PhasePhase>>,
    query: Query<Entity, (With<Tentacle>, With<Dead>, Or<(With<Extending>, With<Retracting>)>)>,
) {
    if query.is_empty() {
        next_state.set(GameState::DecayPhase);
        next_local_state.set(PhasePhase::Idle);
    }
}
