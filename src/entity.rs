use bevy::prelude::*;

use crate::{
    engine::{
        coords::{GridPos, SyncGridPos},
        mapgen::MapData,
        prop::{PropType, spawn::SpawnPropExt},
        rendering::macro_map::MacroMapEntity,
    },
    faction::{BuildingColor, FactionId},
};

#[derive(Component)]
pub struct DynamicObject;

#[derive(Component)]
pub struct Human;

#[derive(Component)]
pub struct Animal;

#[derive(Component)]
pub struct House
{
    pub tier: u32,
}

pub fn spawn_human(
    commands: &mut Commands,
    prop_type: PropType,
    pos: GridPos,
    faction: Option<FactionId>,
)
{
    let base = (
        Human,
        DynamicObject,
        SyncGridPos,
        MacroMapEntity { color: [255, 255, 255, 255] },
    );

    match faction
    {
        Some(fid) => commands.spawn_prop(prop_type, pos, 1, (base, fid)),
        None => commands.spawn_prop(prop_type, pos, 1, base),
    }
}

pub fn spawn_building(
    commands: &mut Commands,
    prop_type: PropType,
    pos: GridPos,
    variation: u32,
    faction: Option<FactionId>,
)
{
    let tier = match prop_type
    {
        PropType::HouseTier0 => 0,
        PropType::HouseTier1 => 1,
        PropType::HouseTier2 => 2,
        PropType::HouseTier3 => 3,
        PropType::HouseTier4 => 4,
        PropType::HouseTier5 => 5,
        PropType::HouseTier6 => 6,
        _ => 0,
    };
    let house = House { tier };

    match faction
    {
        Some(fid) =>
        {
            commands.spawn_prop(prop_type, pos, variation, (BuildingColor::default(), fid, house));
        },
        None =>
        {
            commands.spawn_prop(prop_type, pos, variation, (BuildingColor::default(), house));
        },
    }
}

pub fn spawn_animal(commands: &mut Commands, pos: Vec2, map_data: &MapData)
{
    // Fake animal to test the macro map update.
    commands.spawn((
        Animal,
        GridPos(map_data.world_to_grid(pos)),
        Transform::from_translation(Vec3::new(pos.x, pos.y, 1.0)),
        DynamicObject,
        SyncGridPos,
        MacroMapEntity { color: [255, 255, 255, 255] },
    ));
}

pub fn update_dyn_objects(
    mut dyn_query: Query<&mut Transform, With<DynamicObject>>,
    time: Res<Time>,
)
{
    for mut transform in dyn_query.iter_mut()
    {
        transform.translation += Vec3::new(3.0, 3.0, 0.0) * time.delta_secs();
    }
}

pub struct EntityPlugin;

impl Plugin for EntityPlugin
{
    fn build(&self, app: &mut App)
    {
        app.add_systems(Update, update_dyn_objects);
    }
}
