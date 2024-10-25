use std::time::Duration;

use bevy::{prelude::*, time::Stopwatch};
use bevy_kira_audio::prelude::*;
use blenvy::{BlueprintAnimationPlayerLink, BlueprintAnimations, BlueprintInfo};

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);

use crate::{
    block::Block,
    decay_phase::{DarkFigureBody, Decayed},
    music::{BackgroundMusic, Music},
    GameState,
};

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
        app.insert_resource(TotalScore(0))
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (score, update_score_text, button_system).run_if(in_state(GameState::ScoringPhase)),
            )
            .add_systems(
                OnEnter(GameState::ScoringPhase),
                (hide_dark_figure, show_text, start_scoring_loop),
            )
            .add_systems(OnExit(GameState::ScoringPhase), cleanup);
    }
}

fn setup() {}

fn hide_dark_figure(
    animations: Query<(&BlueprintAnimationPlayerLink, &BlueprintAnimations), With<DarkFigureBody>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (link, animations) in animations.iter() {
        let (mut animation_player, mut transition) = animation_players.get_mut(link.0).unwrap();
        if let Some(emerge_animation) = animations.named_indices.get("hide") {
            println!("THING");
            transition.play(
                &mut animation_player,
                *emerge_animation,
                std::time::Duration::from_secs(1),
            );
        }
    }
}
fn start_scoring_loop(
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
            .play(music.scoring_overlay.clone())
            .start_from(cur)
            .fade_in(AudioTween::linear(Duration::from_secs(1)))
            .looped()
            .handle();
        background.1 = Some(handle);
    }
}

fn show_text(mut commands: Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            },
            UiStuff,
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "0",
                    TextStyle {
                        font_size: 128.0,
                        ..default()
                    },
                )
                .with_text_justify(JustifyText::Center),
                ScoreText,
            ));
            parent
                .spawn(ButtonBundle {
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
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Replay",
                        TextStyle {
                            font_size: 33.0,
                            color: Color::srgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ));
                });
        });
}

fn update_score_text(mut query: Query<&mut Text, With<ScoreText>>, score: Res<TotalScore>) {
    for mut text in &mut query {
        text.sections[0].value = format!("{}", score.0);
    }
}

fn score(
    mut commands: Commands,
    query: Query<Entity, (With<Decayed>, Without<Scored>)>,
    mut stopwatch: Local<Stopwatch>,
    time: Res<Time>,
    mut score: ResMut<TotalScore>,
) {
    stopwatch.tick(time.delta());
    if stopwatch.elapsed().as_secs_f32() < 0.1 {
        return;
    }
    stopwatch.reset();
    for entity in &query {
        commands.entity(entity).insert(Scored);
        score.0 += 1;
        return;
    }
}

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut score: ResMut<TotalScore>,
) {
    for (interaction, mut color, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                next_state.set(GameState::BuildPhase);
                score.0 = 0;
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}

fn cleanup(
    mut commands: Commands,
    query: Query<Entity, Or<(With<Block>, With<UiStuff>, With<BlueprintInfo>)>>,
) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}
