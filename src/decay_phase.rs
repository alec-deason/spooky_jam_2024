use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};
use std::time::Duration;

use bevy_kira_audio::prelude::*;
use blenvy::{BlueprintAnimationPlayerLink, BlueprintAnimations};

use crate::{
    block::{WeirdMachine, AnchorState, Anchors, Block, Conductor, DecayedRepresentation, DisasterTarget},
    environmental_decoration::{Sky, Star},
    music::{BackgroundMusic, Music},
    block_pool::BlockPoolResident,
    CameraScale, GameState, MousePos, SNAP_DISTANCE,
};

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct NeedsDecay;

#[derive(Component)]
pub struct Decayed;
#[derive(Component, Reflect, Copy, Clone, Debug)]
#[reflect(Component)]
pub struct Eye;

#[derive(Component, Reflect, Copy, Clone, Debug)]
#[reflect(Component)]
pub enum Disaster {
    Lightning,
    Fire,
}

#[derive(Component)]
struct ScreenFlash(bevy::time::Stopwatch, std::time::Duration);

#[derive(Resource)]
struct SparkSound(Option<Handle<AudioInstance>>);

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

#[derive(Component, Reflect, Copy, Clone, Debug)]
#[reflect(Component)]
pub struct DarkFigureBody;

#[derive(Component, Reflect, Copy, Clone, Debug)]
#[reflect(Component)]
pub struct SkyTentacle;

#[derive(Component)]
struct ActiveTentacle;

#[derive(Component)]
struct Lightning(Vec<Entity>);

pub struct DecayPhasePlugin;

impl Plugin for DecayPhasePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SparkSound(None))
            .add_plugins(MaterialPlugin::<LineMaterial>::default())
            .register_type::<Disaster>()
            .register_type::<Eye>()
            .register_type::<DecayedRepresentation>()
            .register_type::<SkyTentacle>()
            .register_type::<DarkFigureBody>()
            .insert_state(PhasePhase::Running)
            .add_systems(
                PostUpdate,
                hide_dark_figure.run_if(not(in_state(crate::GameState::DecayPhase))),
            )
            .add_systems(Update, (screen_flash, apply_decay))
            .add_systems(
                OnEnter(crate::GameState::DecayPhase),
                (
                    setup_phase,
                    start_decay_loop,
                    |mut next_state: ResMut<NextState<PhasePhase>>| {
                        next_state.set(PhasePhase::Running)
                    },
                ),
            )
            .add_systems(
                Update,
                (
                    extinguish_eyes,
                    dark_figure_animation_control,
                    spawn_dark_figure,
                    maintain_active_tentacle,
                    check_completion,
                    activate_disaster,
                )
                    .run_if(
                        in_state(PhasePhase::Running)
                            .and_then(in_state(crate::GameState::DecayPhase)),
                    ),
            )
            .add_systems(
                PostUpdate,
                (targeting).run_if(
                    in_state(PhasePhase::Running).and_then(in_state(crate::GameState::DecayPhase)),
                ),
            );
    }
}

fn setup_phase(
    mut commands: Commands,
    query: Query<Entity, With<Eye>>,
) {
    for e in &query {
        commands.entity(e).remove::<ActiveTentacle>().insert(Disaster::Lightning);
    }
}

fn hide_dark_figure(mut body: Query<&mut Visibility, With<DarkFigureBody>>, mut done: Local<bool>) {
    if !*done {
        for mut visibility in &mut body {
            *done = true;
            *visibility = Visibility::Hidden;
        }
    }
}

fn spawn_dark_figure(
    body: Query<Entity, With<DarkFigureBody>>,
    children: Query<&Children>,
    mut visibility: Query<&mut Visibility>,
) {
    for entity in &body {
        for entity in std::iter::once(entity).chain(children.iter_descendants(entity)) {
            if let Ok(mut vis) = visibility.get_mut(entity) {
                *vis = Visibility::Visible;
            }
        }
    }
}

fn start_decay_loop(
    mut background: ResMut<BackgroundMusic>,
    music: Res<Music>,
    audio: Res<Audio>,
    mut instances: ResMut<Assets<AudioInstance>>,
) {
    if let Some(player) = background.1.as_ref().and_then(|h| instances.get_mut(h)) {
        player.stop(AudioTween::linear(Duration::from_secs(1)));
    }
    if let Some(cur) = audio.state(&background.0).position() {
        let handle = audio
            .play(music.decay_overlay.clone())
            .start_from(cur)
            .fade_in(AudioTween::linear(Duration::from_secs(2)))
            .looped()
            .handle();
        background.1 = Some(handle);
    }
}

fn dark_figure_animation_control(
    animations: Query<(&BlueprintAnimationPlayerLink, &BlueprintAnimations), With<DarkFigureBody>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    music: Res<crate::music::Music>,
    audio: Res<Audio>,
) {
    for (link, animations) in animations.iter() {
        let (mut animation_player, mut transition) = animation_players.get_mut(link.0).unwrap();
        if let Some(idle_animation) = animations.named_indices.get("dark_figure_idle") {
            if let Some(emerge_animation) = animations.named_indices.get("emerge") {
                if !animation_player.is_playing_animation(*idle_animation) {
                    if !animation_player.is_playing_animation(*emerge_animation) {
                        transition.play(
                            &mut animation_player,
                            *emerge_animation,
                            std::time::Duration::ZERO,
                        );
                        audio.play(music.dark_figure_hit.clone());
                    } else if animation_player.all_finished() {
                        transition
                            .play(
                                &mut animation_player,
                                *idle_animation,
                                std::time::Duration::ZERO,
                            )
                            .repeat();
                    }
                }
            }
        }
    }
}

fn extinguish_eyes(
    mut query: Query<&mut Sky, (With<Star>, With<Eye>)>,
    mut removed: RemovedComponents<ActiveTentacle>,
    time: Res<Time>,
) {
    for entity in removed.read() {
        if let Ok(mut sky) = query.get_mut(entity) {
            let now = time.elapsed();
            *sky = Sky::Transition {
                start_color: Color::WHITE,
                end_color: Color::WHITE,
                start_time: now,
                end_time: now + std::time::Duration::from_millis(125),
                start_star_brightness: sky.current_star_brightness(now),
                end_star_brightness: 0.1,
            };
        }
    }
}

fn maintain_active_tentacle(
    mut commands: Commands,
    active: Query<Entity, With<ActiveTentacle>>,
    waiting: Query<Entity, With<Disaster>>,
) {
    if active.is_empty() {
        for entity in &waiting {
            commands
                .entity(entity)
                .insert(ActiveTentacle)
                .remove::<Disaster>();
            return;
        }
    }
}

fn targeting(
    mut commands: Commands,
    mouse_pos: Res<MousePos>,
    targets: Query<(Entity, &GlobalTransform, &DisasterTarget)>,
    conductors: Query<(Entity, &GlobalTransform, Option<&WeirdMachine>), With<Conductor>>,
    mut strikes: Query<(Entity, &mut Lightning)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
    tentacles: Query<&GlobalTransform, With<ActiveTentacle>>,
    camera_scale: Res<CameraScale>,
    audio: Res<Audio>,
    music: Res<Music>,
    mut spark: ResMut<SparkSound>,
    mut instances: ResMut<Assets<AudioInstance>>,
) {
    if let Some(tentacle_transform) = tentacles.iter().next() {
        let mut snapped = None;

        let mut maybe_pos = Vec3::new(mouse_pos.0.x, mouse_pos.0.y, -3.2);
        maybe_pos.x = mouse_pos.0.x;
        maybe_pos.y = mouse_pos.0.y;

        let mut min_distance = std::f32::INFINITY;
        for (target_entity, target_transform, disaster_target) in &targets {
            if Disaster::Lightning.compatible(&*disaster_target) {
                let d = maybe_pos - target_transform.translation();
                let dist = d.length();
                if dist < min_distance && dist < SNAP_DISTANCE * camera_scale.0 {
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
                point_count: 1,
                ..default()
            };
            let mut targets = vec![(snapped, maybe_pos.clone(), 1.0)];
            let mut total_travel = 15.0;
            let mut did_work = true;
            while did_work && total_travel > 0.0 {
                did_work = false;
                let mut min_dist = std::f32::INFINITY;
                let mut closest = None;
                for (entity, transform, maybe_machine) in &conductors {
                    if targets.iter().any(|(e, _, _)| *e == entity) {
                        continue;
                    }
                    let d = (targets[targets.len() - 1].1 - transform.translation()).length();
                    if d < 5.0 && d < min_dist {
                        min_dist = d;
                        let (_, rotation, translation) = transform.to_scale_rotation_translation();
                        closest = Some((entity, translation, rotation, maybe_machine.is_some()));
                    }
                }
                if let Some((entity, translation, rotation, is_machine)) = closest {
                    did_work = true;
                    targets.push((entity, translation, total_travel/15.0));
                    if is_machine {
                        total_travel += 15.0;
                        targets.push((entity, translation + rotation.mul_vec3(Vec3::new(5.0, 0.0, 0.0)), total_travel/15.0));
                    } else {
                        total_travel -= min_dist;
                    }
                }
            }
            material.points[0] = tentacle_transform.translation().extend(1.0);
            for i in 0..14.min(targets.len()) {
                material.points[i + 1] = targets[i].1.extend(targets[i].2);
                material.point_count += 1;
            }
            let mut targets: Vec<Entity> = targets.into_iter().map(|(e, _, _)| e).collect();
            targets.insert(0, snapped);
            for (entity, mut lightning) in &mut strikes {
                lightning.0 = targets.clone();
                commands.entity(entity).insert((MaterialMeshBundle {
                    mesh: meshes.add(Rectangle {
                        half_size: Vec2::new(100.0, 100.0),
                    }),
                    material: materials.add(material.clone()),
                    ..default()
                },));
                if spark.0.is_none() {
                    spark.0 = Some(
                        audio
                            .play(music.spark.clone())
                            .start_from(1.2)
                            .linear_fade_in(Duration::from_millis(250))
                            .with_volume(0.125)
                            .loop_from(1.2)
                            .loop_until(9.0)
                            .handle(),
                    );
                }
                found = true;
                break;
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
                    Lightning(targets),
                ));
                if spark.0.is_none() {
                    spark.0 = Some(
                        audio
                            .play(music.spark.clone())
                            .start_from(1.2)
                            .linear_fade_in(Duration::from_millis(250))
                            .with_volume(0.125)
                            .loop_from(1.2)
                            .loop_until(9.0)
                            .handle(),
                    );
                }
            }
        } else {
            for (entity, _transform) in &strikes {
                commands.entity(entity).despawn_recursive();
            }
            if let Some(player) = spark.0.take().and_then(|h| instances.get_mut(&h)) {
                player.stop(AudioTween::linear(Duration::from_millis(250)));
            }
        }
    }
}

fn activate_disaster(
    mut commands: Commands,
    query: Query<(Entity, &Lightning)>,
    blocks: Query<Entity, (With<Block>, Without<NeedsDecay>, Without<Decayed>)>,
    parents: Query<&Parent>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut anchors: Query<&mut Anchors>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tentacles: Query<(Entity, &GlobalTransform), With<ActiveTentacle>>,
    audio: Res<Audio>,
    music: Res<Music>,
    mut spark: ResMut<SparkSound>,
    mut instances: ResMut<Assets<AudioInstance>>,
) {
    if let Some((tentacle_entity, _tentacle_transform)) = tentacles.iter().next() {
        if mouse_button_input.just_released(MouseButton::Left) {
            commands.entity(tentacle_entity).remove::<ActiveTentacle>();
            audio.play(music.thunder.clone());
            if let Some(player) = spark.0.take().and_then(|h| instances.get_mut(&h)) {
                player.stop(AudioTween::linear(Duration::from_millis(250)));
            }
            commands
                .spawn(PbrBundle {
                    mesh: meshes.add(Rectangle {
                        half_size: Vec2::new(100.0, 100.0),
                    }),
                    material: materials.add(StandardMaterial {
                        base_color: Color::srgba(0.0, 0.0, 0.0, 1.0),
                        emissive: Color::srgb(1000.0, 1000.0, 1000.0).into(),
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    }),
                    ..default()
                })
                .insert(ScreenFlash(default(), std::time::Duration::from_millis(75)));
            let mut done = std::collections::HashSet::new();
            for (entity, lightning) in &query {
                for targeted_entity in &lightning.0 {
                    for ancestor in std::iter::once(*targeted_entity)
                        .chain(parents.iter_ancestors(*targeted_entity))
                    {
                        if let Ok(mut anchors) = anchors.get_mut(ancestor) {
                            for (_, _, anchor_state, _) in &mut anchors.0 {
                                if let AnchorState::Occupied(entity) = *anchor_state {
                                    if done.contains(&entity) {
                                        continue;
                                    }
                                    *anchor_state = AnchorState::Blocked(entity);
                                    if blocks.contains(entity) {
                                        commands
                                            .entity(entity)
                                            .insert(NeedsDecay);

                                        done.insert(entity);
                                    }
                                }
                            }
                        }
                        if done.contains(&ancestor) {
                            continue;
                        }
                        if blocks.contains(ancestor) {
                            commands
                                .entity(ancestor)
                                .insert(NeedsDecay);
                            done.insert(ancestor);
                            break;
                        }
                    }
                }
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

fn apply_decay(
    mut commands: Commands,
    query: Query<Entity, With<NeedsDecay>>,
    children: Query<&Children>,
    mut material_handle: Query<&mut Handle<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for needy_entity in &query {
        commands.entity(needy_entity).remove::<NeedsDecay>().remove::<Anchors>().insert(Decayed);
        for entity in std::iter::once(needy_entity).chain(children.iter_descendants(needy_entity)) {
            commands.entity(entity).remove::<DisasterTarget>().remove::<Conductor>();
            if let Some(material) = material_handle.get(entity).ok().and_then(|h| materials.get_mut(h)) {
                let mut hsv: Hsva = material.emissive.into();
                material.emissive = hsv.with_saturation(0.2).with_value(0.2).into();
            }
        }
    }
}

fn check_completion(
    mut next_state: ResMut<NextState<GameState>>,
    mut next_local_state: ResMut<NextState<PhasePhase>>,
    tentacles: Query<Entity, Or<(With<ActiveTentacle>, With<Disaster>)>>,
    blocks: Query<Entity, (With<Block>, Without<Decayed>, Without<BlockPoolResident>)>,
) {
    if tentacles.is_empty() || blocks.is_empty() {
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

fn screen_flash(
    mut commands: Commands,
    mut query: Query<(Entity, &mut ScreenFlash, &Handle<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    for (entity, mut flash, handle) in &mut query {
        flash.0.tick(time.delta());
        if let Some(material) = materials.get_mut(handle) {
            let t = flash.1.as_secs_f32() / flash.0.elapsed().as_secs_f32();
            if flash.0.elapsed().as_secs_f32() >= flash.1.as_secs_f32() {
                commands.entity(entity).despawn_recursive();
            }
            material.base_color = Color::srgba(1.0, 1.0, 1.0, t);
        }
    }
}
