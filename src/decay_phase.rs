use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::{MeshVertexBufferLayoutRef, PrimitiveTopology},
        render_asset::RenderAssetUsages,
        render_resource::{
            AsBindGroup, PolygonMode, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError,
        },
    },
};

use blenvy::{
    BlueprintInfo, GameWorldTag,
    HideUntilReady, SpawnBlueprint,
};
use bevy_mod_picking::prelude::*;

use crate::{SNAP_DISTANCE, GameState, SavedPosition, MousePos, Spawner, Spawned, SpawnedFrom, DISASTERS, block::{DisasterTarget, DecayedRepresentation, Block}, lift_component, Lift};


#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct DisasterSpawner;

#[derive(Component)]
pub struct Decayed;

#[derive(Component, Reflect, Copy, Clone, Debug)]
#[reflect(Component)]
pub enum Disaster {
    Lightning,
    Fire,
}

#[derive(Copy, Clone, Component)]
struct Targeting;

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
    Idle,
}

#[derive(Resource)]
struct LightningStrikes(u32);

#[derive(Component)]
struct Lightning(Vec<Entity>);

pub struct DecayPhasePlugin;

impl Plugin for DecayPhasePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(LightningStrikes(3))
            .add_plugins(MaterialPlugin::<LineMaterial>::default())
            .register_type::<Disaster>()
            .register_type::<DisasterSpawner>()
            .register_type::<DecayedRepresentation>()
            .insert_state(PhasePhase::Running)
            .add_systems(OnEnter(crate::GameState::DecayPhase), |mut next_state: ResMut<NextState<PhasePhase>>| { next_state.set(PhasePhase::Running) })
            .add_systems(Update, (lift_component::<Disaster>, check_completion, activate_disaster).run_if(in_state(PhasePhase::Running).and_then(in_state(crate::GameState::DecayPhase))))
            .add_systems(PostUpdate, targeting.run_if(in_state(PhasePhase::Running).and_then(in_state(crate::GameState::DecayPhase))))
            ;
    }
}

fn targeting(
    mut commands: Commands,
    mouse_pos: Res<MousePos>,
    targets: Query<(Entity, &GlobalTransform, &DisasterTarget)>,
    mut strikes: Query<(Entity, &mut Lightning)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
) {
    let mut snapped = None;

    let mut maybe_pos = Vec3::new(mouse_pos.0.x, mouse_pos.0.y, -3.2);
    maybe_pos.x = mouse_pos.0.x;
    maybe_pos.y = mouse_pos.0.y;

    let mut min_distance = std::f32::INFINITY;
    for (target_entity, target_transform, disaster_target) in &targets {
        if Disaster::Lightning.compatible(&*disaster_target) {
            let d = maybe_pos-target_transform.translation();
            let dist = d.length();
            if dist < min_distance && dist < SNAP_DISTANCE*crate::CAMERA_SCALE {
                min_distance = dist;
                maybe_pos.x = mouse_pos.0.x - d.x;
                maybe_pos.y = mouse_pos.0.y - d.y;
                snapped = Some(target_entity);
            }
        }
    }



    if let Some(snapped) = snapped {
        maybe_pos.z = 0.0;
        let mut found = false;
        let mut material = LineMaterial {
            point_count: 2,
            ..default()
        };
        material.points[0] = Vec4::new(0.0, 100.0, 0.0, 0.0);
        material.points[1] = Vec4::new(maybe_pos.x, maybe_pos.y, 0.0, 0.0);
        for (entity, mut lightning) in &mut strikes {
            commands.entity(entity).insert((
                MaterialMeshBundle {
                    /*
                    mesh: meshes.add(LineStrip {
                        points: vec![
                            Vec3::new(0.0, 100.0, 0.0),
                            Vec3::new(maybe_pos.x, maybe_pos.y, 0.0),
                        ],
                    }),
                    */
                    mesh: meshes.add(Rectangle {
                        half_size: Vec2::new(100.0, 100.0),
                    }),
                    material: materials.add(material.clone()),
                    ..default()
                },
                Lightning(vec![snapped]),
            ));
            found = true;
        }
        if !found {
            commands.spawn((
                MaterialMeshBundle {
                    mesh: meshes.add(Rectangle {
                        half_size: Vec2::new(100.0, 100.0),
                    }),
                    material: materials.add(material),
                    ..default()
                },
                Lightning(vec![snapped]),
            ));
        }
    } else {
        for (entity, _transform) in &strikes {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn activate_disaster(
    mut commands: Commands,
    query: Query<(Entity, &Lightning)>,
    blocks: Query<(&Transform, &DecayedRepresentation)>,
    parents: Query<&Parent>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut strikes: ResMut<LightningStrikes>,
) {

    if strikes.0 > 0 && mouse_button_input.just_released(MouseButton::Left) {
        strikes.0 -= 1;
        for (entity, lightning) in &query {
            for targeted_entity in &lightning.0 {
                for ancestor in parents.iter_ancestors(*targeted_entity) {
                    if let Ok((transform, decayed)) = blocks.get(ancestor) {
                        commands.spawn((
                            BlueprintInfo::from_path(&format!("levels/{}", decayed.0)),
                            transform.clone(),
                            SpawnBlueprint,
                            HideUntilReady,
                            GameWorldTag,
                            Block,
                            Decayed,
                        ));
                        commands.entity(ancestor).despawn_recursive();
                        break;
                    }
                }
            }
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn check_completion(
    mut next_state: ResMut<NextState<GameState>>,
    mut next_local_state: ResMut<NextState<PhasePhase>>,
    mut strikes: ResMut<LightningStrikes>,
) {
    if strikes.0 == 0 {
        next_state.set(GameState::ScoringPhase);
        next_local_state.set(PhasePhase::Idle);
    }
}

#[derive(Asset, TypePath, Default, AsBindGroup, Debug, Clone)]
struct LineMaterial {
    #[uniform(100)]
    points: [Vec4; 16],
    #[uniform(100)]
    point_count: u32,
}

impl Material for LineMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/lightning.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
