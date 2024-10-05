use bevy::{
    prelude::*,
};

mod build_phase;
mod block;

fn main() {
    App::new()
        .add_plugins(build_phase::BuildPhasePlugin)
        .run();
}
