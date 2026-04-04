use bevy::prelude::*;

use crate::engine::{
    coords::{GridPos, SyncGridPos, world_to_grid},
    prop::*,
    rendering::MacroMapDot,
    spawn::SpawnPropExt,
};

#[derive(Component)]
pub struct DynamicObject;

#[derive(Component)]
pub struct Human
{
    pub color: [u8; 3],
}

#[derive(Component)]
pub struct Animal;

pub fn spawn_human(commands: &mut Commands, pos: GridPos, color: [u8; 3])
{
    commands.spawn_prop(
        PropType::HumanAnimation,
        pos,
        (
            Human { color },
            DynamicObject,
            SyncGridPos,
            MacroMapDot { color: [color[0], color[1], color[2], 255] },
        ),
    );
}

pub fn spawn_animal(commands: &mut Commands, pos: Vec2)
{
    // Fake animal to test the macro map update.
    commands.spawn((
        Animal,
        GridPos(world_to_grid(pos)),
        Transform::from_translation(Vec3::new(pos.x, pos.y, 1.0)),
        DynamicObject,
        SyncGridPos,
        MacroMapDot { color: [255, 255, 255, 255] },
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
