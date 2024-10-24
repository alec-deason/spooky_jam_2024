use std::collections::HashMap;

use bevy::prelude::*;

use blenvy::{
    BlueprintAnimationPlayerLink, BlueprintAnimations, BlueprintInfo, GameWorldTag,
    HideUntilReady, SpawnBlueprint, BlueprintInstanceReady,
};

use crate::{BLOCKS, DECAYED};

pub struct BlockPoolPlugin;

#[derive(Component)]
pub struct BlockPool;

#[derive(Component)]
pub struct BlockPoolResident(pub String);
#[derive(Component)]
pub struct TempBlockPoolResident(pub String);

#[derive(Component)]
pub struct DecayedPoolResident(pub String);
#[derive(Component)]
pub struct TempDecayedPoolResident(pub String);

#[derive(Resource)]
struct Pool(Entity);


impl Plugin for BlockPoolPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup)
            .add_systems(Update, (maintain_pool, add_resident_tag))
        ;
    }
}

fn setup(
    mut commands: Commands,
) {
    let e = commands.spawn((
        Name::new("Pool"),
        BlockPool,
        TransformBundle::from_transform(Transform::from_translation(Vec3::new(10000.0, 10000.0, 100000.0))),
        VisibilityBundle {
            visibility: Visibility::Hidden,
            ..default()
        },
    )).id();
    commands.insert_resource(Pool(e));
}

fn maintain_pool(
    mut commands: Commands,
    block_query: Query<&BlockPoolResident>,
    decayed_query: Query<&DecayedPoolResident>,
    pool: Res<Pool>,
) {
    let mut count = HashMap::with_capacity(BLOCKS.len());
    for resident in &block_query {
        *count.entry(resident.0.clone()).or_insert(0) += 1;
    }

    for path in &BLOCKS {
        if count.get(*path).copied().unwrap_or(0) < 4 {
            let mut transform = Transform::from_translation(Vec3::new(10000.0, 10000.0, 100000.0));
            if path.contains("reversable") && fastrand::f32() > 0.5 {
                transform = transform.with_scale(Vec3::new(-1.0, 1.0, 1.0));
            };
            let id = commands.spawn((
                transform,
                BlueprintInfo::from_path(path),
                Visibility::Hidden,
                SpawnBlueprint,
                HideUntilReady,
                GameWorldTag,
                TempBlockPoolResident(path.to_string()),
            )).id();
            //commands.entity(pool.0).push_children(&[id]);
        }
    }

    let mut count = HashMap::with_capacity(DECAYED.len());
    for resident in &decayed_query {
        *count.entry(resident.0.clone()).or_insert(0) += 1;
    }

    for path in &DECAYED {
        if count.get(*path).copied().unwrap_or(0) < 4 {
            let id = commands.spawn((
                BlueprintInfo::from_path(path),
                Visibility::Hidden,
                Transform::from_translation(Vec3::new(10000.0, 10000.0, 100000.0)),
                SpawnBlueprint,
                HideUntilReady,
                GameWorldTag,
                TempDecayedPoolResident(path.to_string()),
            )).id();
            commands.entity(pool.0).push_children(&[id]);
        }
    }
}

fn add_resident_tag(
    mut commands: Commands,
    block_query: Query<(Entity, &TempBlockPoolResident), With<BlueprintInstanceReady>>,
    decayed_query: Query<(Entity, &TempDecayedPoolResident), With<BlueprintInstanceReady>>,
) {
    for (entity, resident) in &block_query {
        commands.entity(entity).remove::<TempBlockPoolResident>().insert(BlockPoolResident(resident.0.clone()));
    }
    for (entity, resident) in &decayed_query {
        commands.entity(entity).remove::<TempDecayedPoolResident>().insert(DecayedPoolResident(resident.0.clone()));
    }
}
