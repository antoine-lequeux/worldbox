use bevy::{prelude::*, window::PresentMode};
use worldbox::{
    engine::{EnginePlugin, GridPos, prop::PropType},
    entity::{EntityPlugin, spawn_building, spawn_human},
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

fn setup(mut commands: Commands, mut factions: ResMut<FactionRegistry>)
{
    let empire = factions.add("Empire", [15, 59, 125]);
    let forest = factions.add("Forest Clan", [34, 120, 56]);

    spawn_human(
        &mut commands,
        PropType::HumanImperialWalking,
        GridPos::new(5, 9),
        [15, 59, 125],
        Some(empire),
    );
    spawn_human(
        &mut commands,
        PropType::HumanImperialWalking,
        GridPos::new(8, 9),
        [34, 120, 56],
        Some(forest),
    );
    spawn_human(
        &mut commands,
        PropType::HumanImperialWalking,
        GridPos::new(11, 9),
        [200, 200, 200],
        None,
    );

    spawn_building(&mut commands, PropType::HouseTier1, GridPos::new(513, 10), 0, Some(empire));
    spawn_building(&mut commands, PropType::HouseTier1, GridPos::new(517, 10), 0, Some(forest));
    spawn_building(&mut commands, PropType::HouseTier1, GridPos::new(521, 10), 0, None);
}
