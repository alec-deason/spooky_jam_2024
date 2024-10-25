use bevy::{prelude::*, render::primitives::Aabb};
use bevy_mod_picking::prelude::*;
use std::collections::HashMap;

use crate::{block_pool::BlockPool, lift_component};

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Block;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub enum DisasterTarget {
    All,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Conductor;

#[derive(Component, Reflect, Copy, Clone)]
#[reflect(Component)]
pub struct NoCollide;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct FoundationAnchor;

#[derive(Copy, Clone, Debug, Component, Reflect, PartialEq)]
pub enum AnchorColor {
    Up,
    Down,
    DecorationUp,
    DecorationDown,
    None,
}

impl AnchorColor {
    pub fn compatible(&self, other: Self) -> bool {
        match self {
            AnchorColor::Up => other == AnchorColor::Down,
            AnchorColor::Down => other == AnchorColor::Up,
            AnchorColor::DecorationUp => other == AnchorColor::DecorationDown,
            AnchorColor::DecorationDown => other == AnchorColor::DecorationUp,
            AnchorColor::None => false,
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Anchor(AnchorColor);

#[derive(Copy, Clone, Debug, PartialEq, Reflect)]
pub enum AnchorState {
    Clear,
    Occupied(Entity),
    Blocked(Entity),
}

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub struct Anchors(pub Vec<(Vec3, AnchorColor, AnchorState, bool)>);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct MouseAnchor;

#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct DecayedRepresentation(pub String);

#[derive(Component)]
struct Collider(Box<dyn parry3d::shape::Shape>);

#[derive(Component)]
pub struct InCollision;

#[derive(Component)]
struct ProcessedCollider;

pub struct BlockPlugin;

impl Plugin for BlockPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Anchor>()
            .register_type::<FoundationAnchor>()
            .register_type::<Anchors>()
            .register_type::<Conductor>()
            .register_type::<NoCollide>()
            .register_type::<AnchorColor>()
            .register_type::<DecayedRepresentation>()
            .register_type::<DisasterTarget>()
            .register_type::<MouseAnchor>()
            .register_type::<Pickable>()
            .register_type::<PickingInteraction>()
            .register_type::<PickSelection>()
            .register_type::<PickHighlight>()
            .register_type::<Block>()
            .add_systems(PreUpdate, test_colliders)
            .add_systems(
                Update,
                (
                    add_colliders,
                    configure_anchors,
                    lift_component::<DecayedRepresentation>,
                    lift_component::<NoCollide>,
                ),
            )
            .add_systems(PostUpdate, check_anchor_clearance)
        ;
    }
}

fn configure_anchors(
    mut commands: Commands,
    anchors: Query<(Entity, &Transform, &Anchor, Option<&FoundationAnchor>)>,
    parent_query: Query<&Parent>,
    pool: Query<Entity, With<BlockPool>>,
    mut composite_anchors: Query<(Option<&mut Anchors>, &Transform)>,
) {
    let mut to_insert = HashMap::new();
    for (base_entity, base_transform, anchor, foundation) in &anchors {
        commands.entity(base_entity).remove::<Anchor>();
        let parent_entity = parent_query
            .iter_ancestors(base_entity)
            .filter(|e| !pool.contains(*e))
            .last()
            .unwrap();
        if let Ok((maybe_anchors, parent_transform)) = composite_anchors.get_mut(parent_entity) {
            let mut translation = base_transform.translation * parent_transform.scale;
            translation.x = (translation.x).round();
            translation.y = (translation.y).round();
            translation.z = (translation.z).round();

            if let Some(mut anchors) = maybe_anchors {
                anchors.0.push((
                    translation,
                    anchor.0,
                    AnchorState::Clear,
                    foundation.is_some(),
                ));
            } else {
                to_insert.entry(parent_entity).or_insert(vec![]).push((
                    translation,
                    anchor.0,
                    AnchorState::Clear,
                    foundation.is_some(),
                ));
            }
        }
    }

    for (entity, anchors) in to_insert {
        commands.entity(entity).insert(Anchors(anchors));
    }
}

fn add_colliders(
    mut commands: Commands,
    query: Query<
        (Entity, &Aabb),
        (
            With<crate::block::Block>,
            Without<ProcessedCollider>,
            Without<NoCollide>,
        ),
    >,
) {
    for (entity, aabb) in &query {
        commands
            .entity(entity)
            .insert(ProcessedCollider)
            .insert(Collider(Box::new(parry3d::shape::Cuboid {
                half_extents: [
                    aabb.half_extents.x * 0.9,
                    aabb.half_extents.y * 0.9,
                    aabb.half_extents.z * 0.9,
                ]
                .into(),
            })));
    }
}

fn test_colliders(mut commands: Commands, query: Query<(Entity, &GlobalTransform, &Collider)>) {
    let mut collisions: HashMap<Entity, Vec<Entity>> = HashMap::new();
    for [(entity_a, transform_a, collider_a), (entity_b, transform_b, collider_b)] in
        query.iter_combinations()
    {
        collisions.entry(entity_a).or_default();
        collisions.entry(entity_b).or_default();
        let t_a = transform_a.translation();
        let t_b = transform_b.translation();
        if parry3d::query::intersection_test(
            &[t_a.x, t_a.y, t_a.z].into(),
            collider_a.0.as_ref(),
            &[t_b.x, t_b.y, t_b.z].into(),
            collider_b.0.as_ref(),
        )
        .unwrap()
        {
            collisions.entry(entity_a).or_default().push(entity_b);
            collisions.entry(entity_b).or_default().push(entity_a);
        }
    }
    for (entity, collisions) in collisions {
        if collisions.is_empty() {
            commands.entity(entity).remove::<InCollision>();
        } else {
            commands.entity(entity).insert(InCollision);
        }
    }
}

fn check_anchor_clearance(
    mut anchors: Query<(Entity, &GlobalTransform, &mut Anchors)>,
    blocks: Query<(Entity, &GlobalTransform, &Collider)>,
) {
    for (anchor_entity, base_transform, mut anchors) in &mut anchors {
        for (anchor_transform, anchor_color, ref mut anchor_state, _) in anchors.0.iter_mut() {
            if *anchor_color == AnchorColor::Up
                && matches!(anchor_state, AnchorState::Clear | AnchorState::Blocked(_))
            {
                *anchor_state = AnchorState::Clear;
                let t_a =
                    base_transform.translation() + *anchor_transform + Vec3::new(0.0, 1.0, 0.0);
                let anchor_collider = parry3d::shape::Cuboid {
                    half_extents: [0.5, 0.5, 0.5].into(),
                };
                for (block_entity, block_transform, block_collider) in &blocks {
                    if anchor_entity == block_entity {
                        continue;
                    }
                    let t_b = block_transform.translation();
                    if parry3d::query::intersection_test(
                        &[t_a.x, t_a.y, t_a.z].into(),
                        &anchor_collider,
                        &[t_b.x, t_b.y, t_b.z].into(),
                        block_collider.0.as_ref(),
                    )
                    .unwrap()
                    {
                        *anchor_state = AnchorState::Blocked(block_entity);
                    }
                }
            }
        }
    }
}
