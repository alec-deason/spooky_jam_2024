use std::collections::HashMap;

use bevy::prelude::*;

use blenvy::{BlueprintInfo, BlueprintInstanceReady, GameWorldTag, HideUntilReady, SpawnBlueprint};

use crate::BLOCKS;

pub struct BlockPoolPlugin;

#[derive(Component)]
pub struct BlockPool;

#[derive(Component)]
pub struct BlockPoolResident(pub String);
#[derive(Component)]
pub struct TempBlockPoolResident(pub String);

#[derive(Resource)]
struct Pool(Entity);

impl Plugin for BlockPoolPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (maintain_pool, add_resident_tag));
    }
}

fn setup(mut commands: Commands) {
    let e = commands
        .spawn((
            Name::new("Pool"),
            BlockPool,
            TransformBundle::from_transform(Transform::from_translation(Vec3::new(
                10000.0, 10000.0, 100000.0,
            ))),
            VisibilityBundle {
                visibility: Visibility::Hidden,
                ..default()
            },
        ))
        .id();
    commands.insert_resource(Pool(e));
}

fn maintain_pool(
    mut commands: Commands,
    block_query: Query<&BlockPoolResident>,
    temp_block_query: Query<&TempBlockPoolResident>,
    pool: Res<Pool>,
) {
    let mut count = HashMap::with_capacity(BLOCKS.len());
    for resident in &block_query {
        *count.entry(resident.0.clone()).or_insert(0) += 1;
    }
    for resident in &temp_block_query {
        *count.entry(resident.0.clone()).or_insert(0) += 1;
    }

    for (path, _) in &BLOCKS {
        if count.get(*path).copied().unwrap_or(0) < 3 {
            let mut transform = Transform::from_translation(Vec3::new(10000.0, 10000.0, 100000.0));
            if path.contains("reversable") && fastrand::f32() > 0.5 {
                transform = transform.with_scale(Vec3::new(-1.0, 1.0, 1.0));
            };
            commands.spawn((
                transform,
                BlueprintInfo::from_path(path),
                Visibility::Hidden,
                SpawnBlueprint,
                HideUntilReady,
                GameWorldTag,
                TempBlockPoolResident(path.to_string()),
            ));
            break;
        }
    }
}

fn add_resident_tag(
    mut commands: Commands,
    block_query: Query<(Entity, &TempBlockPoolResident), With<BlueprintInstanceReady>>,
) {
    for (entity, resident) in &block_query {
        commands
            .entity(entity)
            .remove::<TempBlockPoolResident>()
            .insert(BlockPoolResident(resident.0.clone()));
    }
}
