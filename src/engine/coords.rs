use bevy::prelude::*;

use crate::engine::mapgen::MapData;

// Tile position on the map grid.
#[derive(Component, Clone, Copy, PartialEq, Eq, Deref, DerefMut, Debug)]
pub struct GridPos(pub IVec2);

impl GridPos
{
    // Create a grid position from integer coordinates.
    pub fn new(x: i32, y: i32) -> Self
    {
        return Self(IVec2 { x, y });
    }
}

// Marker component: entities with this have their GridPos kept in sync with their Transform.
#[derive(Component)]
pub struct SyncGridPos;

// Updates GridPos from Transform for all entities marked with SyncGridPos.
pub fn sync_grid_positions(
    map_data: Res<MapData>,
    mut query: Query<(&mut GridPos, &Transform), (With<SyncGridPos>, Changed<Transform>)>,
)
{
    for (mut grid_pos, transform) in &mut query
    {
        let new_pos = map_data.world_to_grid(transform.translation.xy());
        if grid_pos.0 != new_pos
        {
            grid_pos.0 = new_pos;
        }
    }
}
