use crate::engine::{mapgen::MapData, tile::TileRegistry};

// Bitmask values for each cardinal direction.
const N: u8 = 1;
const E: u8 = 2;
const S: u8 = 4;
const W: u8 = 8;

// Tile-coordinate offsets for each cardinal direction (N, E, S, W).
pub const CARDINAL_OFFSETS: [(i32, i32); 4] = [
    (0, 1),  // N
    (1, 0),  // E
    (0, -1), // S
    (-1, 0), // W
];

// Bitmask for each direction index, matching CARDINAL_OFFSETS order.
const CARDINAL_BITS: [u8; 4] = [N, E, S, W];

// For a tile at (gx, gy), returns the overlay sprite index for a higher-priority
// neighbor in the given cardinal direction. Returns None if no overlay is needed.
pub fn compute_overlay_for_dir(
    gx: u32,
    gy: u32,
    dir: usize,
    map_data: &MapData,
    tile_registry: &TileRegistry,
) -> Option<u32>
{
    let center_type = map_data.get_tile(gx, gy);
    let center_def = tile_registry.tiles.get(&center_type)?;

    let (dx, dy) = CARDINAL_OFFSETS[dir];
    let nx = gx as i32 + dx;
    let ny = gy as i32 + dy;

    if nx < 0
        || ny < 0
        || nx >= map_data.width_tiles() as i32
        || ny >= map_data.height_tiles() as i32
    {
        return None;
    }

    let neighbor_type = map_data.get_tile(nx as u32, ny as u32);
    if neighbor_type == center_type
    {
        return None;
    }

    let neighbor_priority = tile_registry
        .tiles
        .get(&neighbor_type)
        .map(|d| d.priority)
        .unwrap_or(0);

    if neighbor_priority <= center_def.priority
    {
        return None;
    }

    let foreign_def = tile_registry.tiles.get(&neighbor_type)?;
    return Some(foreign_def.blob_offset + CARDINAL_BITS[dir] as u32);
}
