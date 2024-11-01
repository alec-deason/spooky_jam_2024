use std::{
    time::Duration,
    collections::{HashMap, BinaryHeap},
};

use bevy::{
    prelude::*,
    time::common_conditions::on_timer,
};
use blenvy::*;
use bevy_mod_picking::prelude::*;

use crate::{
    Spawned, SpawnedFrom, build_phase::{Retracting, OnTentacle},
    environmental_decoration::TimeOfDay,
    build_phase::AwaitingPlacement,
    block::Block,
    block_pool::BlockPoolResident,
    DECORATIONS,
};


#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct CrowParams {
    avoidance: f32,
    alignment: f32,
    cohesion: f32,
    friction: f32,
    avoid_distance: f32,
    speed: f32,
    max_speed: f32,
    target: Vec3,
}


#[derive(Component, Reflect)]
#[reflect(Component)]
struct CrowPerch;

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Grab(pub Entity);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Crow;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Flying;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Perching;

#[derive(Component)]
pub struct CrowPickupTarget;

#[derive(Component)]
pub struct CrowTakeawayTarget;

#[derive(Component)]
struct CrowAssigned;


#[derive(Component, Reflect, PartialEq, Debug)]
#[reflect(Component)]
pub enum Employed {
    ExitingToDeliver(Entity, Vec3),
    ExitingToPickup(Entity),
    Delivering(Entity, Vec3),
    PickingUp(Entity),
    ReturningToFlock,
}

impl Employed {
    fn next(&self) -> Option<Self> {
        match self {
            Employed::ExitingToDeliver(e, v) => Some(Employed::Delivering(*e,*v)),
            Employed::ExitingToPickup(e) => Some(Employed::PickingUp(*e)),
            Employed::Delivering(..) => Some(Employed::ReturningToFlock),
            Employed::PickingUp(..) => Some(Employed::ReturningToFlock),
            Employed::ReturningToFlock => None,
        }
    }
}

pub struct CrowPlugin;

impl Plugin for CrowPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<CrowParams>()
            .register_type::<CrowPerch>()
            .register_type::<Flying>()
            .register_type::<Perching>()
            .insert_resource(CrowParams {
                avoidance: 0.3,
                alignment: 0.3,
                cohesion: 0.3,
                friction: 0.99,
                avoid_distance: 4.0,
                speed: 0.05,
                max_speed: 0.75,
                target: Vec3::ZERO,
            })
            .add_systems(
                OnEnter(crate::GameState::BuildPhase),
                spawn_crows,
            )
            .add_systems(
                PreUpdate,
                (control_crow_visual, perturb_crows, steer_crows, steer_employed_crows, assign_crow_jobs, cleanup_crow_jobs).run_if(on_timer(Duration::from_millis(16)))
            )
            .add_systems(
                Update,
                (control_perched_crows, move_crows).run_if(on_timer(Duration::from_millis(16)))
            )
        ;
    }
}

fn spawn_crows(
    mut commands: Commands,
) {
    for _ in 0..10 {
        let transform = Transform::from_translation(Vec3::new(fastrand::i32(-60..-40) as f32, fastrand::i32(0..23) as f32, -7.1));
        commands.spawn((
            transform,
            BlueprintInfo::from_path("levels/crow.glb"),
            SpawnBlueprint,
            HideUntilReady,
            GameWorldTag,
            Velocity(Vec2::ZERO),
            Crow,
        ));
    }
}

fn move_crows(
    mut query: Query<(&mut Transform, &Velocity)>,
) {
    for (mut t, v) in &mut query {
        t.translation.x += v.0.x;
        t.translation.y += v.0.y;
    }
}

struct NeighborhoodEntry(f32, Entity, Vec3, Vec2);
impl PartialEq for NeighborhoodEntry {
    fn eq(&self, other: &Self) -> bool {
        ordered_float::OrderedFloat(self.0) == ordered_float::OrderedFloat(other.0)
    }
}
impl Eq for NeighborhoodEntry {}

impl PartialOrd for NeighborhoodEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for NeighborhoodEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self == other {
            std::cmp::Ordering::Equal
        } else {
            ordered_float::OrderedFloat(self.0).cmp(&ordered_float::OrderedFloat(other.0))
        }
    }
}

impl From<(f32, Entity, Vec3, Vec2)> for NeighborhoodEntry {
    fn from(v: (f32, Entity, Vec3, Vec2)) -> Self {
        NeighborhoodEntry(v.0, v.1, v.2, v.3)
    }
}

struct Neighborhood {
    furthrest: f32,
    entities: BinaryHeap<NeighborhoodEntry>,
}


impl Default for Neighborhood {
    fn default() -> Self {
        Self {
            furthrest: std::f32::INFINITY,
            entities: BinaryHeap::new(),
        }
    }
}

fn steer_crows(
    mut query: Query<(Entity, &Transform, &mut Velocity), Without<Employed>>,
    mut neighborhoods: Local<HashMap<Entity, Neighborhood>>,
    params: Res<CrowParams>,
) {
    for (e,t,_) in &query {
        for (e2, t2, v2) in &query {
            if e==e2 {
                continue
            }
            let d = (t.translation - t2.translation).length();
            let entry = neighborhoods.entry(e).or_default();
            entry.entities.push((d, e2, t2.translation, v2.0).into());
            if entry.entities.len() > 3 {
                entry.entities.pop();
            }
        }
    }

    for (entity, neighborhood) in neighborhoods.drain() {
        let (_, t, mut v) = query.get_mut(entity).unwrap();
        let mut avg_vel = Vec2::ZERO;
        let mut alignment_count = 0;
        let mut center_of_mass = Vec3::ZERO;
        for entry in neighborhood.entities {
            if entry.0 < params.avoid_distance {
                let d = (t.translation - entry.2) * params.avoidance;
                v.0 += d.xy() * params.speed;
            } else {
                alignment_count += 1;
                avg_vel += entry.3;
                center_of_mass += entry.2;
            }
        }
        if alignment_count > 0 {
            let nv = (avg_vel/alignment_count as f32 - v.0) * params.alignment;
            v.0 += nv * params.speed;
        }
        center_of_mass += params.target*1.0;
        alignment_count += 1;
        v.0 += ((center_of_mass/alignment_count as f32 - t.translation).xy()) * params.cohesion * params.speed;
        v.0 *= params.friction;
        v.0 = v.0.clamp(-Vec2::splat(params.max_speed), Vec2::splat(params.max_speed));
    }
}

fn steer_employed_crows(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Velocity, &mut Employed), Without<CrowTakeawayTarget>>,
    mut takeaways: Query<(&mut Transform, &SpawnedFrom), With<CrowTakeawayTarget>>,
    block_pool: Query<(Entity, &BlockPoolResident)>,
    transforms: Query<&GlobalTransform>,
    params: Res<CrowParams>,
) {
    for (crow_entity, mut t, mut v, mut employment) in &mut query {
        let mut speed_mul = 2.5;
        match &*employment {
            Employed::ExitingToDeliver(_, _) | Employed::ExitingToPickup(_) | Employed::ReturningToFlock => {
                if &*employment == &Employed::ReturningToFlock {
                    t.scale = Vec3::splat(5.0);
                    t.translation.z = 1.0;
                } else {
                    t.scale = Vec3::splat(1.0);
                    t.translation.z = -7.1;
                }
                v.0 += (Vec2::new(0.0, 50.0)-t.translation.xy())*params.speed*speed_mul;
                if (t.translation.xy() - Vec2::new(0.0, 50.0)).length() < 10.0 {
                    if let Some(next) = employment.next() {
                        *employment = next;
                    } else {
                        t.scale = Vec3::splat(1.0);
                        t.translation.z = -7.1;
                        commands.entity(crow_entity).remove::<Employed>();
                    }
                }
            }
            Employed::Delivering(_, target) => {
                t.scale = Vec3::splat(5.0);
                t.translation.z = 1.0;
                if (target.xy()-t.translation.xy()).length() > 5.0 {
                    v.0 += (target.xy()-t.translation.xy())*params.speed*(target.xy()-t.translation.xy()).length().powi(2);
                } else {
                    v.0 = Vec2::ZERO;
                }
            }
            Employed::PickingUp(e) => {
                t.scale = Vec3::splat(5.0);
                t.translation.z = 1.0;
                if let Ok((mut takeaway, spawned_from)) = takeaways.get_mut(*e) {
                    if (t.translation.xy() - Vec2::new(0.0, 50.0)).length() < 10.0 {
                        commands.entity(*e).despawn_recursive();
                        commands
                            .entity(spawned_from.0)
                            .remove::<Spawned>()
                            .insert(Retracting);
                        commands.entity(crow_entity).remove::<Grab>();

                        let draw = fastrand::f32();
                        let idx = DECORATIONS
                         .iter()
                         .position(|(_, p)| *p >= draw)
                         .unwrap_or(DECORATIONS.len() - 1);
                        let path = &DECORATIONS[idx].0;
                        let mut found = None;
                        for (entity, resident) in &block_pool {
                         if &resident.0 == path {
                             found = Some(entity);
                             break;
                         }
                        }
                        if let Some(entity) = found {
                         commands
                             .entity(entity)
                             .insert((
                                 OnTentacle,
                                 Block,
                                 Visibility::Visible,
                                 SpawnedFrom(crow_entity),
                                 PickableBundle::default(),
                                 On::<Pointer<DragStart>>::listener_insert(AwaitingPlacement),
                                 On::<Pointer<DragEnd>>::listener_remove::<AwaitingPlacement>(),
                             ))
                             .remove::<BlockPoolResident>();
                         commands.entity(crow_entity).insert(Spawned(entity));
                         let target = Vec3::new(
                             fastrand::i32(-23..22) as f32,
                             fastrand::i32(9..22) as f32,
                             10.0
                         );
                         *employment = Employed::Delivering(entity, target);
                        }
                    } else {
                        v.0 += (Vec2::new(0.0, 50.0)-t.translation.xy())*params.speed*speed_mul;
                    }
                    takeaway.translation = t.translation;
                } else {
                    if let Ok(target) = transforms.get(*e) {
                        let d = target.translation().xy()-t.translation.xy();
                        if d.length() < 4.0 {
                            commands.entity(crow_entity).insert(Grab(*e));
                            v.0 = Vec2::ZERO;
                        } else {
                            v.0 += d*params.speed*speed_mul*(target.translation().xy()-t.translation.xy()).length().powi(2);
                        }
                    }
                }
            }
        }
        v.0 = v.0.clamp(-Vec2::splat(params.max_speed*speed_mul), Vec2::splat(params.max_speed*speed_mul));
    }
}

fn control_crow_visual(
    mut commands: Commands,
    query: Query<(Entity, Option<&Grab>), With<Crow>>,
    takeaways: Query<&Transform, With<CrowTakeawayTarget>>,
    children: Query<&Children>,
    flying: Query<Entity, With<Flying>>,
    perching: Query<Entity, With<Perching>>,
) {
    for (entity, grab) in &query {
        let taking_away = grab.as_ref().and_then(|g| Some(takeaways.contains(g.0))).unwrap_or(false);
        for entity in std::iter::once(entity).chain(children.iter_descendants(entity)) {
            if let Ok(entity) = flying.get(entity) {
                if grab.is_some() && !taking_away {
                    commands.entity(entity).insert(Visibility::Hidden);
                } else {
                    commands.entity(entity).insert(Visibility::Visible);
                }
            }
            if let Ok(entity) = perching.get(entity) {
                if grab.is_some() && !taking_away {
                    commands.entity(entity).insert(Visibility::Visible);
                } else {
                    commands.entity(entity).insert(Visibility::Hidden);
                }
            }
        }
    }
}

fn control_perched_crows(
    mut query: Query<(&mut Transform, &Grab), Without<CrowTakeawayTarget>>,
    children: Query<&Children>,
    perches: Query<&GlobalTransform, With<CrowPerch>>,
    takeaways: Query<&Transform, With<CrowTakeawayTarget>>,
) {
    for (mut crow_t, grabbed) in &mut query {
        if takeaways.contains(grabbed.0) {
            continue
        }
        for entity in std::iter::once(grabbed.0).chain(children.iter_descendants(grabbed.0)) {
            if let Ok(other_t) = perches.get(entity) {
                let (_scale, rotation, translation) = other_t.to_scale_rotation_translation();
                crow_t.translation = translation;
                crow_t.translation.z += 2.0;
                crow_t.rotation = rotation;
                break
            }
        }
    }
}

fn assign_crow_jobs(
    mut commands: Commands,
    jobs: Query<Entity, (With<CrowPickupTarget>, Without<CrowAssigned>)>,
    crows: Query<Entity, (With<Crow>, Without<Employed>)>,
) {
    for entity in &jobs {
        if let Some(crow) = crows.iter().next() {
            commands.entity(entity).insert(CrowAssigned);
            commands.entity(crow).insert(Employed::ExitingToPickup(entity));
        }
    }
}

fn cleanup_crow_jobs(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Employed, &mut Transform, Option<&Velocity>, Option<&Spawned>)>,
    jobs: Query<Entity, Or<(With<CrowPickupTarget>, With<CrowTakeawayTarget>)>>,
) {
    for (e, mut employment, mut t, maybe_v, maybe_spawned) in &mut query {
        match &*employment {
            Employed::ExitingToPickup(target) => {
                if !jobs.contains(*target) {
                    t.scale = Vec3::splat(1.0);
                    t.translation.z = -7.1;
                    commands.entity(e).remove::<Employed>();
                    commands.entity(*target).remove::<CrowAssigned>();
                }
            }
            Employed::PickingUp(target) => {
                if !jobs.contains(*target) {
                    commands.entity(*target).remove::<CrowAssigned>();
                    commands.entity(e).remove::<Grab>();
                    if maybe_v.is_none() {
                        commands.entity(e).insert(Velocity(default()));
                    }
                    *employment = Employed::ReturningToFlock;
                }
            }
            Employed::Delivering(..) => {
                if maybe_spawned.is_none() {
                    *employment = Employed::ReturningToFlock;
                }
            }
            _ => (),
        }
    }
}

fn perturb_crows(
    mut params: ResMut<CrowParams>,
    mut avoidance_target: Local<Option<f32>>,
    time_of_day: Res<TimeOfDay>,
    mut last_time_of_day: Local<TimeOfDay>,
) {
    if avoidance_target.is_none() {
        *avoidance_target = Some(params.avoid_distance);
    }

    if *time_of_day == TimeOfDay::Day {
        if fastrand::f32() < 1.0/(60.0*8.0) || *last_time_of_day != *time_of_day {
            params.target.x = fastrand::i32(-40..34) as f32;
            params.target.y = fastrand::i32(0..23) as f32;
        }

        if *last_time_of_day != *time_of_day {
            params.avoid_distance = avoidance_target.unwrap_or(4.0);
        } else if fastrand::f32() < 1.0/(60.0*8.0) {
            params.avoid_distance = 10.0;
        } else if params.avoid_distance > avoidance_target.unwrap_or(4.0) {
            params.avoid_distance -= 0.05;
        }
    } else {
        params.target.x = 74.0;
        params.target.y = 70.0;
        params.avoid_distance = 40.0;
    }
    *last_time_of_day = *time_of_day;
}
