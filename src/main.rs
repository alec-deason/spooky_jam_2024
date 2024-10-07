use bevy::{
    prelude::*,
};
use blenvy::*;
use bevy_mod_picking::prelude::*;

use bevy_inspector_egui::quick::WorldInspectorPlugin;

mod build_phase;
mod block;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(low_latency_window_plugin()))
        .add_plugins(BlenvyPlugin::default())
        //.add_plugins(WorldInspectorPlugin::new())
        .add_plugins(DefaultPickingPlugins
            .build()
            .disable::<DebugPickingPlugin>())
        .insert_resource(DebugPickingMode::Normal)
        .add_plugins(build_phase::BuildPhasePlugin)
        .run();
}
