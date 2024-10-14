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

use crate::{GameState, block::Block};

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
    query: Query<(Entity, &GlobalTransform), (With<Block>, Without<Scored>)>,
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
        commands
            // Add the bundle specifying the particle system itself.
            .spawn(ParticleSystemBundle {
                transform: transform.clone().into(),
                particle_system: ParticleSystem {
                    max_particles: 10_000,
                    texture: ParticleTexture::Sprite(asset_server.load("px.png")),
                    spawn_rate_per_second: 25.0.into(),
                    initial_speed: JitteredValue::jittered(3.0, -1.0..1.0),
                    lifetime: JitteredValue::jittered(8.0, -2.0..2.0),
                    color: ColorOverTime::Gradient(Curve::new(vec![
                        CurvePoint::new(Color::WHITE, 0.0),
                        CurvePoint::new(Color::srgba(0.0, 0.0, 1.0, 0.0), 1.0),
                    ])),
                    looping: true,
                    system_duration_seconds: 10.0,
                    ..ParticleSystem::default()
                },
                ..ParticleSystemBundle::default()
            })
            // Add the playing component so it starts playing. This can be added later as well.
            .insert(Playing);
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
