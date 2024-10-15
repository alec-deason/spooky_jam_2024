use bevy::{
    prelude::*,
    time::Stopwatch,
    color::palettes::basic::*,
};
use blenvy::BlueprintInfo;
use bevy_particle_systems::*;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

use crate::{GameState, block::Block, decay_phase::Decayed};

pub struct ScoringPhasePlugin;

#[derive(Component)]
struct Scored;

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct UiStuff;

#[derive(Resource)]
struct TotalScore(u32);

impl Plugin for ScoringPhasePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(TotalScore(0))
            .add_systems(Startup, setup)
            .add_systems(Update, (score, update_score_text, button_system).run_if(in_state(GameState::ScoringPhase)))
            .add_systems(OnEnter(GameState::ScoringPhase), show_text)
            .add_systems(OnExit(GameState::ScoringPhase), cleanup)
        ;
    }
}

fn setup(
) {
}

fn show_text(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn((NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        }, UiStuff))
        .with_children(|parent| {
            parent.spawn(
                    (
                    TextBundle::from_section("0", TextStyle { font_size: 128.0, ..default() })
                    .with_text_justify(JustifyText::Center),
                    ScoreText,
                    )
                );
            parent.spawn(ButtonBundle {
                    style: Style {
                        width: Val::Px(150.0),
                        height: Val::Px(65.0),
                        border: UiRect::all(Val::Px(5.0)),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    border_radius: BorderRadius::MAX,
                    background_color: NORMAL_BUTTON.into(),
                    ..default()
                })
                .with_children(|parent| {parent.spawn(TextBundle::from_section(
                    "Button",
                    TextStyle {
                        font_size: 33.0,
                        color: Color::srgb(0.9, 0.9, 0.9),
                        ..default()
                    },
                ));});
        });

}

fn update_score_text(
    mut query: Query<&mut Text, With<ScoreText>>,
    score: Res<TotalScore>,
) {
    for mut text in &mut query {
        text.sections[0].value = format!("{}", score.0);
    }
}

fn score(
    mut commands: Commands,
    query: Query<(Entity, &GlobalTransform), (With<Decayed>, Without<Scored>)>,
    mut stopwatch: Local<Stopwatch>,
    time: Res<Time>,
    mut score: ResMut<TotalScore>,
    asset_server: Res<AssetServer>,
) {
    stopwatch.tick(time.delta());
    if stopwatch.elapsed().as_secs_f32() < 0.1 {
        return
    }
    stopwatch.reset();
    for (entity, transform) in &query {
        score.0 += 1;
        commands.entity(entity).insert(Scored);
        println!("POOP");
                commands.spawn((
            ParticleSystemBundle {
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
                particle_system: ParticleSystem {
                    spawn_rate_per_second: 0.0.into(),
                    max_particles: 1_000,
                    initial_speed: (0.0..300.0).into(),
                    scale: 2.0.into(),
                    velocity_modifiers: vec![
                        VelocityModifier::Drag(0.001.into()),
                        VelocityModifier::Vector(Vec3::new(0.0, -400.0, 0.0).into()),
                    ],
                    color: (BLUE.into()..Color::srgba(1.0, 0.0, 0.0, 0.0)).into(),
                    bursts: vec![ParticleBurst {
                        time: 0.0,
                        count: 1000,
                    }],
                    ..ParticleSystem::oneshot()
                },
                ..default()
            },
            Playing,
        ));
        return
    }
}

fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut next_state: ResMut<NextState<GameState>>,
    mut score: ResMut<TotalScore>,
) {
    for (interaction, mut color, mut border_color, children) in &mut interaction_query {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                next_state.set(GameState::BuildPhase);
                score.0 = 0;
            }
            Interaction::Hovered => {
                text.sections[0].value = "Hover".to_string();
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                text.sections[0].value = "Button".to_string();
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}

fn cleanup(
    mut commands: Commands,
    query: Query<Entity, Or<(With<Block>, With<ParticleSystem>, With<UiStuff>, With<BlueprintInfo>)>>,
) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}
