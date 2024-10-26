use std::{
    time::Duration,
    collections::{HashMap, BinaryHeap},
};

use bevy::{
    prelude::*,
    time::common_conditions::on_timer,
};
use blenvy::*;

use crate::GameState;


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

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Crow;

#[derive(Component)]
pub struct CrowPickupTarget;
#[derive(Component)]
struct CrowAssigned;


#[derive(Component, Reflect, PartialEq)]
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
            .insert_resource(CrowParams {
                avoidance: 0.3,
                alignment: 0.3,
                cohesion: 0.3,
                friction: 0.995,
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
                (perturb_crows, steer_crows, steer_employed_crows, assign_crow_jobs, cleanup_crow_jobs).run_if(on_timer(Duration::from_millis(16)))
            )
            .add_systems(
                Update,
                ( move_crows).run_if(on_timer(Duration::from_millis(16)))
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
    mut query: Query<(Entity, &mut Transform, &mut Velocity, &mut Employed)>,
    transforms: Query<&GlobalTransform>,
    params: Res<CrowParams>,
) {
    for (entity, mut t, mut v, mut employment) in &mut query {
        match &*employment {
            Employed::ExitingToDeliver(_, _) | Employed::ExitingToPickup(_) | Employed::ReturningToFlock => {
                if &*employment == &Employed::ReturningToFlock {
                    t.scale = Vec3::splat(5.0);
                } else {
                    t.scale = Vec3::splat(1.0);
                }
                v.0 += (Vec2::new(0.0, 50.0)-t.translation.xy())*params.speed;
                if (t.translation.xy() - Vec2::new(0.0, 50.0)).length() < 10.0 {
                    if let Some(next) = employment.next() {
                        *employment = next;
                    } else {
                        t.scale = Vec3::splat(1.0);
                        commands.entity(entity).remove::<Employed>();
                    }
                }
            }
            Employed::Delivering(_, target) => {
                t.scale = Vec3::splat(5.0);
                v.0 += (target.xy()-t.translation.xy())*params.speed;
            }
            Employed::PickingUp(e) => {
                t.scale = Vec3::splat(5.0);
                if let Ok(target) = transforms.get(*e) {
                    v.0 += (target.translation().xy()-t.translation.xy())*params.speed;
                }
            }
        }
        v.0 = v.0.clamp(-Vec2::splat(params.max_speed), Vec2::splat(params.max_speed));
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
    mut query: Query<(Entity, &mut Employed, &mut Transform)>,
    jobs: Query<Entity, With<CrowPickupTarget>>,
) {
    for (e, mut employment, mut t) in &mut query {
        match &*employment {
            Employed::ExitingToPickup(target) => {
                if !jobs.contains(*target) {
                    t.scale = Vec3::splat(1.0);
                    commands.entity(e).remove::<Employed>();
                    commands.entity(*target).remove::<CrowAssigned>();
                }
            }
            Employed::PickingUp(target) => {
                if !jobs.contains(*target) {
                    commands.entity(*target).remove::<CrowAssigned>();
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
) {
    if avoidance_target.is_none() {
        *avoidance_target = Some(params.avoid_distance);
    }

    if fastrand::f32() < 1.0/(60.0*8.0) {
        params.target.x = fastrand::i32(-40..34) as f32;
        params.target.y = fastrand::i32(0..23) as f32;
    }

    if fastrand::f32() < 1.0/(60.0*8.0) {
        params.avoid_distance = 10.0;
    } else if params.avoid_distance > avoidance_target.unwrap_or(4.0) {
        params.avoid_distance -= 0.05;
    }
}
