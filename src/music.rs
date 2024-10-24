use std::time::Duration;

use bevy::prelude::*;
use bevy_kira_audio::prelude::{*, AudioSource};

use crate::{CLANKS, SQUELCHES, SPLASHES};

#[derive(Resource)]
pub struct Music {
    pub build_phase: Handle<AudioSource>,
    pub dark_figure_hit: Handle<AudioSource>,
    pub thunder: Handle<AudioSource>,
    pub spark: Handle<AudioSource>,
    pub decay_overlay: Handle<AudioSource>,
    pub scoring_overlay: Handle<AudioSource>,
}
#[derive(Resource)]
pub struct Clanks(pub Vec<Handle<AudioSource>>);
#[derive(Resource)]
pub struct Squelches(pub Vec<Handle<AudioSource>>);
#[derive(Resource)]
pub struct Splashes(pub Vec<Handle<AudioSource>>);

#[derive(Component)]
pub struct FadeIn;

#[derive(Resource)]
pub struct BackgroundMusic(pub Handle<AudioInstance>, pub Option<Handle<AudioInstance>>);

#[derive(Component)]
struct FadeOut;

const FADE_TIME: f32 = 2.0;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
     fn build(&self, app: &mut App) {
         app
             .add_systems(Startup, start_load)
         ;
     }
}

fn start_load(
    mut commands: Commands,
    mut assets: ResMut<AssetServer>,
    audio: Res<Audio>
) {
    let mut clanks = Clanks(vec![]);
    for p in &CLANKS {
        clanks.0.push(assets.load::<AudioSource>(*p));
    }
    commands.insert_resource(clanks);

    let mut squelches = Squelches(vec![]);
    for p in &SQUELCHES {
        squelches.0.push(assets.load::<AudioSource>(*p));
    }
    commands.insert_resource(squelches);

    let mut splashes = Splashes(vec![]);
    for p in &SPLASHES {
        splashes.0.push(assets.load::<AudioSource>(*p));
    }
    commands.insert_resource(splashes);

    let build_phase = assets.load::<AudioSource>("audio/build_phase.ogg");
    let dark_figure_hit = assets.load::<AudioSource>("audio/dark_figure.ogg");
    let decay_overlay = assets.load::<AudioSource>("audio/RuinsMakerLooping.ogg");
    let scoring_overlay = assets.load::<AudioSource>("audio/harp_layer.ogg");
    let thunder = assets.load::<AudioSource>("audio/thunder.ogg");
    let spark = assets.load::<AudioSource>("audio/spark.ogg");
    let id = audio.play(build_phase.clone()).looped().fade_in(AudioTween::new(Duration::from_secs(2), AudioEasing::OutPowi(2))).handle();
    commands.insert_resource(BackgroundMusic(id, None));

    commands.insert_resource(Music { build_phase, dark_figure_hit, decay_overlay, scoring_overlay, thunder, spark });
}
