use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;
use worldbox::{
    engine::{EnginePlugin, GridPos, PropType, SpawnPropExt},
    entity::{EntityPlugin, spawn_animal, spawn_human},
};

fn main()
{
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(TilemapPlugin)
        .add_plugins(EnginePlugin)
        .add_plugins(EntityPlugin)
        .insert_resource(ClearColor(Color::srgb_u8(43, 140, 237)))
        .add_systems(PostStartup, setup)
        .run();
}

fn setup(mut commands: Commands)
{
    spawn_human(&mut commands, GridPos::new(5, 9), [15, 59, 125]);
    spawn_animal(&mut commands, Vec2::new(15.5, 29.6));
    spawn_house(&mut commands, GridPos::new(513, 384));
}

fn spawn_house(commands: &mut Commands, pos: GridPos)
{
    commands.spawn_prop(PropType::House, pos, ());
}
