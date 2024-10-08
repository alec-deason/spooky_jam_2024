use bevy::prelude::*;
use bevy_mod_picking::prelude::*;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Block;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub enum DisasterTarget {
    All
}

#[derive(Copy, Clone, Debug, Component, Reflect, PartialEq)]
pub enum AnchorColor {
    Up,
    Down,
    None,
}

impl AnchorColor {
    pub fn compatible(&self, other: Self) -> bool {
        match self {
            AnchorColor::Up => other == AnchorColor::Down,
            AnchorColor::Down => other == AnchorColor::Up,
            AnchorColor::None => false,
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Anchor(AnchorColor);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Anchors(pub Vec<(Vec3, AnchorColor, Option<Entity>)>);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct MouseAnchor;

pub struct BlockPlugin;

impl Plugin for BlockPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<Anchor>()
            .register_type::<AnchorColor>()
            .register_type::<DisasterTarget>()
            .register_type::<MouseAnchor>()
            .register_type::<Pickable>()
            .register_type::<PickingInteraction>()
            .register_type::<PickSelection>()
            .register_type::<PickHighlight>()
            .register_type::<Block>()
            .add_systems(Update, configure_anchors);
    }
}

fn configure_anchors(mut commands: Commands, anchors: Query<(Entity, &Transform, &Anchor)>, parent_query: Query<&Parent>, mut composite_anchors: Query<(Option<&mut Anchors>, &Transform)>) {
    for (base_entity, base_transform, anchor) in &anchors {
        commands.entity(base_entity).remove::<Anchor>();
        let parent_entity = parent_query.iter_ancestors(base_entity).last().unwrap();
        if let Ok((maybe_anchors, parent_transform)) = composite_anchors.get_mut(parent_entity) {
            //let offset = base_transform.translation() - parent_transform.translation();
            let mut translation = base_transform.translation * parent_transform.scale;

            if let Some(mut anchors) = maybe_anchors {
                anchors.0.push((translation, anchor.0, None));
            } else {
                commands.entity(parent_entity).insert(Anchors(vec![(translation, anchor.0, None)]));
                return;
            }
        }
    }
}
