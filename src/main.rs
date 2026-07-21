use bevy::{prelude::*, window::PresentMode};
use worldbox::{
    engine::EnginePlugin,
    entity::EntityPlugin,
    faction::{FactionPlugin, FactionRegistry},
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
        .add_plugins(FactionPlugin)
        .add_systems(PostStartup, setup)
        .run();
}

fn setup(mut _commands: Commands, mut factions: ResMut<FactionRegistry>)
{
    factions.add("Empire", [15, 59, 125]);
    factions.add("Forest Clan", [34, 120, 56]);
}
