use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;
use worldbox::{camera::*, map::*};

fn main()
{
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(TilemapPlugin)
        .insert_resource(ClearColor(Color::srgb_u8(105, 173, 181)))
        .add_systems(Startup, (setup_chunked_map, setup_camera))
        .add_systems(Update, update_camera)
        .run();
}
