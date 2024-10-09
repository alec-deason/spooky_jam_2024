use bevy::{
    prelude::*,
    time::Stopwatch,
};
use bevy_hanabi::prelude::*;

use crate::{GameState, block::Block};

pub struct ScoringPhasePlugin;

#[derive(Component)]
struct Scored;

#[derive(Component)]
struct ScoreText;

#[derive(Resource)]
struct TotalScore(u32);

#[derive(Resource)]
pub struct HanabiHandle(Handle<bevy_hanabi::EffectAsset>);

impl Plugin for ScoringPhasePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(TotalScore(0))
            .add_systems(Startup, setup)
            .add_systems(Update, (score, update_score_text).run_if(in_state(GameState::ScoringPhase)))
            .add_systems(OnEnter(GameState::ScoringPhase), show_text)
        ;
    }
}

fn setup(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let mut gradient2 = Gradient::new();
    gradient2.add_key(0.0, Vec4::new(0.0, 0.7, 0.0, 1.0));
    gradient2.add_key(1.0, Vec4::splat(0.0));

    let writer2 = ExprWriter::new();
    let age2 = writer2.lit(0.).expr();
    let init_age2 = SetAttributeModifier::new(Attribute::AGE, age2);
    let lifetime2 = writer2.lit(5.).expr();
    let init_lifetime2 = SetAttributeModifier::new(Attribute::LIFETIME, lifetime2);
    let init_pos2 = SetPositionSphereModifier {
        center: writer2.lit(Vec3::ZERO).expr(),
        radius: writer2.lit(5.).expr(),
        dimension: ShapeDimension::Volume,
    };
    let init_vel2 = SetVelocitySphereModifier {
        center: writer2.lit(Vec3::ZERO).expr(),
        speed: writer2.lit(2.).expr(),
    };
    let effect_handle = effects.add(
        EffectAsset::new(
            vec![32768],
            Spawner::once(1000.0.into(), true),
            writer2.finish(),
        )
        .with_name("emit:once")
        .init(init_pos2)
        .init(init_vel2)
        .init(init_age2)
        .init(init_lifetime2)
        .render(ColorOverLifetimeModifier {
            gradient: gradient2,
        }),
    );
  commands.insert_resource(HanabiHandle(effect_handle));
}

fn show_text(
    mut commands: Commands,
) {
    commands.spawn(
        (
        TextBundle::from("From an &str into a TextBundle with the default font!").with_style(
            Style {
                position_type: PositionType::Absolute,
                bottom: Val::Percent(50.0),
                left: Val::Percent(50.0),
                ..default()
            },
        ).with_text_justify(JustifyText::Center),
        ScoreText,
        )
    );

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
    handle: Res<HanabiHandle>,
    query: Query<(Entity, &GlobalTransform), (With<Block>, Without<Scored>)>,
    mut stopwatch: Local<Stopwatch>,
    time: Res<Time>,
    mut score: ResMut<TotalScore>,
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
        .spawn(ParticleEffectBundle {
            effect: ParticleEffect::new(handle.0.clone()),
            transform: transform.clone().into(),
            ..Default::default()
        });
        return
    }
}
