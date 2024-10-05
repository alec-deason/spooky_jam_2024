use bevy::prelude::*;
use blenvy::*;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct Anchor {
}

pub struct BuildPhasePlugin;

impl Plugin for BuildPhasePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins((DefaultPlugins, BlenvyPlugin::default()))
            .register_type::<Anchor>()
            .insert_resource(AmbientLight {
                color: Color::WHITE,
                brightness: 2000.,
            })
            .add_systems(Update, spam_anchor)
            .add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.7, 100.0, 100.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            ..default()
        },
    ));

    commands.spawn((
        BlueprintInfo::from_path("levels/World.glb"),
        SpawnBlueprint,
        HideUntilReady,
        GameWorldTag,
    ));

    //commands.spawn(SceneBundle {
    //    scene: asset_server
    //        .load(GltfAssetLabel::Scene(0).from_asset("blocks/basic.glb")),
    //    ..default()
    //}).insert(crate::block::Block);
}

fn spam_anchor(query: Query<&Anchor>) {
    for anchor in &query {
        println!("SPAM");
    }
}
