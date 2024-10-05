use bevy::prelude::*;

#[derive(Component)]
pub struct Block;

#[derive(Resource)]
pub struct BlockPack(Handle<Gltf>);

pub fn load_block(
    mut commands: Commands,
    ass: Res<AssetServer>,
) {
    let gltf = ass.load("blocks/basic.glb");
    commands.insert_resource(BlockPack(gltf));
}

pub fn spawn_block(
    mut commands: Commands,
    my: Res<BlockPack>,
    assets_gltf: Res<Assets<Gltf>>,
) {
    // if the GLTF has loaded, we can navigate its contents
    if let Some(gltf) = assets_gltf.get(&my.0) {
        // spawn the first scene in the file
        commands.spawn(SceneBundle {
            scene: gltf.scenes[0].clone(),
            ..Default::default()
        });

        // spawn the scene named "YellowCar"
        //ommands.spawn(SceneBundle {
        //    scene: gltf.named_scenes["YellowCar"].clone(),
        //    transform: Transform::from_xyz(1.0, 2.0, 3.0),
        //    ..Default::default()
        //});

        // PERF: the `.clone()`s are just for asset handles, don't worry :)
    }
}
