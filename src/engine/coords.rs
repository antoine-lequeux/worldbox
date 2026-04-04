use bevy::prelude::*;

use crate::engine::consts::*;

#[derive(Component, Clone, Copy, PartialEq, Eq, Deref, DerefMut, Debug)]
pub struct GridPos(pub IVec2);

impl GridPos
{
    pub fn new(x: i32, y: i32) -> Self
    {
        return Self(IVec2 { x, y });
    }
}

#[derive(Component)]
pub struct SyncGridPos;

pub fn world_to_grid(world: Vec2) -> IVec2
{
    let map_half_w = (MAP_WIDTH * CHUNK_SIZE) as f32 / 2.0;
    let map_half_h = (MAP_HEIGHT * CHUNK_SIZE) as f32 / 2.0;
    let ts = TILE_SIZE as f32;

    return IVec2::new(
        ((world.x / ts) + map_half_w).floor() as i32,
        ((world.y / ts) + map_half_h).floor() as i32,
    );
}

pub fn grid_to_world(tile: IVec2) -> Vec2
{
    let map_half_w = (MAP_WIDTH * CHUNK_SIZE) as f32 / 2.0;
    let map_half_h = (MAP_HEIGHT * CHUNK_SIZE) as f32 / 2.0;
    let ts = TILE_SIZE as f32;

    return Vec2::new(
        (tile.x as f32 - map_half_w) * ts + ts * 0.5,
        (tile.y as f32 - map_half_h) * ts + ts * 0.5,
    );
}

pub fn grid_to_prop_world(pos: IVec2, size_tiles: UVec2) -> Vec3
{
    let base = grid_to_world(pos);
    let ts = TILE_SIZE as f32;
    let offset =
        Vec2::new((size_tiles.x as f32 - 1.0) * ts / 2.0, (size_tiles.y as f32 - 1.0) * ts / 2.0);
    return (base + offset).extend(PROP_Z);
}

pub fn sync_grid_positions(
    mut query: Query<(&mut GridPos, &Transform), (With<SyncGridPos>, Changed<Transform>)>,
)
{
    for (mut grid_pos, transform) in &mut query
    {
        let new_pos = world_to_grid(transform.translation.xy());
        if grid_pos.0 != new_pos
        {
            grid_pos.0 = new_pos;
        }
    }
}
