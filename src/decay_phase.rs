use bevy::prelude::*;
use blenvy::{
    BlueprintAnimationPlayerLink, BlueprintAnimations, BlueprintInfo, GameWorldTag,
    HideUntilReady, SpawnBlueprint,
};
use bevy_mod_picking::prelude::*;

use crate::{SNAP_DISTANCE, SavedPosition, MousePos, Spawner, Spawned, SpawnedFrom, DISASTERS, block::DisasterTarget};

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct DisasterSpawner;

#[derive(Component, Reflect, Copy, Clone, Debug)]
#[reflect(Component)]
pub enum Disaster {
    Lightning,
    Fire,
}

#[derive(Copy, Clone, Component)]
struct Targeting;

#[derive(Copy, Clone, Component)]
struct LiftDisaster;

#[derive(Copy, Clone, Component)]
struct Activate;

#[derive(Copy, Clone, Component)]
struct Targeted(Entity);

impl Disaster {
    fn compatible(&self, target: &DisasterTarget) -> bool {
        match target {
            DisasterTarget::All => true,
        }
    }
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum PhasePhase {
    Running,
    ShuttingDown,
}
pub struct DecayPhasePlugin;

impl Plugin for DecayPhasePlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<Disaster>()
            .register_type::<DisasterSpawner>()
            .insert_state(PhasePhase::Running)
            .add_systems(OnEnter(crate::GameState::DecayPhase), |mut next_state: ResMut<NextState<PhasePhase>>| { next_state.set(PhasePhase::Running) })
            .add_systems(Update, (activate_disaster, lift_disaster, spawn_disaster).run_if(in_state(PhasePhase::Running)).run_if(in_state(crate::GameState::DecayPhase)))
            ;
    }
}

fn spawn_disaster(
    mut commands: Commands,
    spawners: Query<(Entity, &GlobalTransform), (With<Spawner>, With<DisasterSpawner>, Without<Spawned>)>,
) {
    for (spawner_entity, spawner_transform) in &spawners {
        let path = &DISASTERS[fastrand::usize(0..DISASTERS.len())];
        let mut transform:Transform = spawner_transform.clone().into();
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
            LiftDisaster,
            On::<Pointer<DragStart>>::run(|event: Res<ListenerInput<Pointer<DragStart>>>, mut commands: Commands, transform: Query<&Transform>| {
                let transform = transform.get(event.listener()).unwrap();
                commands.entity(event.listener()).insert(SavedPosition(transform.clone())).insert(Targeting).insert(Pickable::IGNORE);
            }),
            On::<Pointer<DragEnd>>::run(drop_disaster),
            On::<Pointer<Drag>>::run(targeting),
        )).id();
        commands.entity(spawner_entity).insert(Spawned(block_entity));
    }
}

fn drop_disaster(
    mut commands: Commands,
    event: Res<ListenerInput<Pointer<DragEnd>>>,
    targeted: Query<Entity, With<Targeted>>,
    mut saved_position: Query<(&mut Transform, &SavedPosition)>,
) {
    let entity = event.listener();
    commands.entity(entity).remove::<Targeting>().insert(Pickable::default());
    if targeted.contains(entity) {
        commands.entity(entity).insert(Activate);
    } else if let Ok((mut transform, saved_position)) = saved_position.get_mut(entity) {
        *transform = saved_position.0.clone();
    }
}

fn targeting(
    mut commands: Commands,
    event: Res<ListenerInput<Pointer<Drag>>>,
    mut query: Query<(Entity, &mut Transform, &Disaster), With<Targeting>>,
    mouse_pos: Res<MousePos>,
    others: Query<(Entity, &GlobalTransform, &DisasterTarget)>
) {
    if let Ok((entity, mut transform, disaster)) = query.get_mut(event.listener()) {
        let mut snapped = None;

        transform.rotation = Quat::IDENTITY;
        let mut maybe_pos = transform.translation.clone();
        maybe_pos.x = mouse_pos.0.x;
        maybe_pos.y = mouse_pos.0.y;

        for (other_entity, other_transform, disaster_target) in &others {
            if disaster.compatible(&*disaster_target) {
                let d = maybe_pos-other_transform.translation();
                if d.length() < SNAP_DISTANCE*crate::CAMERA_SCALE {
                    maybe_pos.x -= d.x;
                    maybe_pos.y -= d.y;
                    snapped = Some(Targeted(other_entity));
                    break;
                }
            }
        }

        transform.translation.x = maybe_pos.x;
        transform.translation.y = maybe_pos.y;

        if let Some(snapped) = snapped {
            commands.entity(entity).insert(snapped);
        } else {
            commands.entity(entity).remove::<Targeted>();
        }
    }
}

fn lift_disaster(
    mut commands: Commands,
    query: Query<(Entity, &Disaster), Without<BlueprintInfo>>,
    main_blueprint: Query<Entity, With<BlueprintInfo>>,
    parents: Query<&Parent>,
) {
    for (src_entity, disaster) in &query {
        commands.entity(src_entity).remove::<Disaster>();
        for ancestor in parents.iter_ancestors(src_entity) {
            if main_blueprint.contains(ancestor) {
                commands.entity(ancestor).insert(disaster.clone());
            }
        }
    }
}

fn activate_disaster(
    mut commands: Commands,
    query: Query<(Entity, &Targeted, &Disaster, &SpawnedFrom), With<Activate>>
) {
    for (entity, targeted, disaster, spawned_from) in &query {
        println!("POP");
        commands.entity(entity).despawn_recursive();
        commands.entity(spawned_from.0).remove::<Spawned>();
    }
}
