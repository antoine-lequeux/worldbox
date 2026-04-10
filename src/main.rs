use bevy::{prelude::*, window::PresentMode};
use worldbox::{
    engine::{
        EnginePlugin, GridPos, MapData,
        prop::{PropType, spawn::SpawnPropExt},
    },
    entity::{EntityPlugin, spawn_animal, spawn_human},
};

fn main()
{
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(EnginePlugin)
        .add_plugins(EntityPlugin)
        .add_systems(PostStartup, setup)
        .run();
}

fn setup(mut commands: Commands, map_data: Res<MapData>)
{
    spawn_human(&mut commands, GridPos::new(5, 9), [15, 59, 125]);
    spawn_animal(&mut commands, Vec2::new(15.5, 29.6), &map_data);
    spawn_house(&mut commands, GridPos::new(513, 384));
}

fn spawn_house(commands: &mut Commands, pos: GridPos)
{
    commands.spawn_prop(PropType::House, pos, ());
}
