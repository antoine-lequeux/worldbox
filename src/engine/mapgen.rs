use std::process::{Command, Stdio};

use bevy::prelude::*;

use crate::engine::{
    consts::{CHUNK_SIZE, MAP_HEIGHT, MAP_SEED, MAP_WIDTH, NUM_CONTINENTS, PROP_Z, TILE_SIZE},
    tile::TileType,
};

// Central resource holding the full tile map, dimensions, and dirty flags.
#[derive(Resource)]
pub struct MapData
{
    // Size of one tile in pixels.
    pub tile_size: u32,
    // Number of tiles per chunk side.
    pub chunk_size: u32,
    // Flat array of tile types, row-major (y * width + x).
    tiles: Vec<TileType>,
    // Map dimensions in tiles.
    width: u32,
    height: u32,
    // Map dimensions in chunks.
    pub chunks_x: u32,
    pub chunks_y: u32,
    // Per-chunk dirty flag: set when any tile inside changes.
    pub dirty_chunks: Vec<bool>,
    // Separate dirty flags for the macro map (not cleared by autotile rebuild).
    pub macro_dirty_chunks: Vec<bool>,
}

impl MapData
{
    // Returns the map width in tiles.
    pub fn width_tiles(&self) -> u32
    {
        return self.width;
    }

    // Returns the map height in tiles.
    pub fn height_tiles(&self) -> u32
    {
        return self.height;
    }

    // Returns the tile type at the given tile coordinates.
    pub fn get_tile(&self, x: u32, y: u32) -> TileType
    {
        return self.tiles[(y * self.width + x) as usize];
    }

    // Sets a tile and marks the containing chunk dirty. Returns true if the type changed.
    // Also dirties neighbour chunks at boundaries (overlay edges depend on neighbours).
    pub fn set_tile(&mut self, x: u32, y: u32, tile_type: TileType) -> bool
    {
        let idx = (y * self.width + x) as usize;
        if self.tiles[idx] == tile_type
        {
            return false;
        }
        self.tiles[idx] = tile_type;
        let cx = x / self.chunk_size;
        let cy = y / self.chunk_size;
        let ci = (cy * self.chunks_x + cx) as usize;
        self.dirty_chunks[ci] = true;
        self.macro_dirty_chunks[ci] = true;

        // Dirty neighbour chunks when the tile sits on a chunk boundary.
        let lx = x % self.chunk_size;
        let ly = y % self.chunk_size;
        if lx == 0 && cx > 0
        {
            self.dirty_chunks[(cy * self.chunks_x + (cx - 1)) as usize] = true;
        }
        if lx == self.chunk_size - 1 && cx + 1 < self.chunks_x
        {
            self.dirty_chunks[(cy * self.chunks_x + (cx + 1)) as usize] = true;
        }
        if ly == 0 && cy > 0
        {
            self.dirty_chunks[((cy - 1) * self.chunks_x + cx) as usize] = true;
        }
        if ly == self.chunk_size - 1 && cy + 1 < self.chunks_y
        {
            self.dirty_chunks[((cy + 1) * self.chunks_x + cx) as usize] = true;
        }
        return true;
    }

    // Converts a world-space position to tile-grid coordinates.
    pub fn world_to_grid(&self, world_pos: Vec2) -> IVec2
    {
        let half_w = (self.width as f32 * self.tile_size as f32) / 2.0;
        let half_h = (self.height as f32 * self.tile_size as f32) / 2.0;
        let x = ((world_pos.x + half_w) / self.tile_size as f32).floor() as i32;
        let y = ((world_pos.y + half_h) / self.tile_size as f32).floor() as i32;
        return IVec2::new(x, y);
    }

    // Converts a grid position to world-space center for a prop of the given size.
    pub fn grid_to_prop_world(&self, grid: IVec2, size_tiles: UVec2) -> Vec3
    {
        let half_w = (self.width as f32 * self.tile_size as f32) / 2.0;
        let half_h = (self.height as f32 * self.tile_size as f32) / 2.0;
        let ts = self.tile_size as f32;
        let x = (grid.x as f32 * ts) - half_w + (size_tiles.x as f32 * ts / 2.0);
        let y = (grid.y as f32 * ts) - half_h + (size_tiles.y as f32 * ts / 2.0);
        return Vec3::new(x, y, PROP_Z);
    }

    // Reads and clears the dirty flag for the given chunk. Returns true if it was dirty.
    pub fn take_chunk_dirty(&mut self, cx: u32, cy: u32) -> bool
    {
        let idx = (cy * self.chunks_x + cx) as usize;
        let was = self.dirty_chunks[idx];
        self.dirty_chunks[idx] = false;
        return was;
    }

    // Reads and clears the macro dirty flag for the given chunk.
    pub fn take_macro_chunk_dirty(&mut self, cx: u32, cy: u32) -> bool
    {
        let idx = (cy * self.chunks_x + cx) as usize;
        let was = self.macro_dirty_chunks[idx];
        self.macro_dirty_chunks[idx] = false;
        return was;
    }

    // Returns the world-space bottom-left corner of a chunk.
    pub fn chunk_world_origin(&self, cx: u32, cy: u32) -> Vec2
    {
        let half_w = (self.width as f32 * self.tile_size as f32) / 2.0;
        let half_h = (self.height as f32 * self.tile_size as f32) / 2.0;
        let ts = self.tile_size as f32;
        return Vec2::new(
            cx as f32 * self.chunk_size as f32 * ts - half_w,
            cy as f32 * self.chunk_size as f32 * ts - half_h,
        );
    }
}

// Maps a numeric tile ID from terrain.py output to a TileType.
fn tile_type_from_id(id: u8) -> TileType
{
    return match id
    {
        0 => TileType::Ocean,
        1 => TileType::DeepWater,
        2 => TileType::ShallowWater,
        3 => TileType::Sand,
        4 => TileType::PlainGrass,
        5 => TileType::ForestGrass,
        6 => TileType::Hill,
        7 => TileType::Mountain,
        _ => TileType::Ocean,
    };
}

// Runs terrain.py as a subprocess and parses its output into a MapData resource.
fn generate_map() -> MapData
{
    let width = MAP_WIDTH * CHUNK_SIZE;
    let height = MAP_HEIGHT * CHUNK_SIZE;

    info!(
        "Generating world map ({}x{} tiles, {}x{} chunks, seed={})...",
        width, height, MAP_WIDTH, MAP_HEIGHT, MAP_SEED
    );

    #[cfg(windows)]
    let python = "py";
    #[cfg(not(windows))]
    let python = "python3";

    let child = Command::new(python)
        .arg("terrain.py")
        .arg(MAP_SEED.to_string())
        .arg(width.to_string())
        .arg(height.to_string())
        .arg(NUM_CONTINENTS.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to run terrain.py");

    let output = child
        .wait_with_output()
        .expect("Failed to wait on terrain.py");

    if !output.status.success()
    {
        panic!("terrain.py failed with exit code {:?}", output.status.code());
    }

    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8 from terrain.py");

    let mut tiles = Vec::with_capacity((width * height) as usize);
    for line in stdout.lines()
    {
        for ch in line.bytes()
        {
            if ch.is_ascii_digit()
            {
                tiles.push(tile_type_from_id(ch - b'0'));
            }
        }
    }

    assert_eq!(
        tiles.len(),
        (width * height) as usize,
        "Tilemap size mismatch: expected {}, got {}",
        width * height,
        tiles.len()
    );

    info!("World generation complete.");

    return MapData {
        tile_size: TILE_SIZE,
        chunk_size: CHUNK_SIZE,
        tiles,
        width,
        height,
        chunks_x: MAP_WIDTH,
        chunks_y: MAP_HEIGHT,
        dirty_chunks: vec![false; (MAP_WIDTH * MAP_HEIGHT) as usize],
        macro_dirty_chunks: vec![false; (MAP_WIDTH * MAP_HEIGHT) as usize],
    };
}

pub struct MapGenPlugin;

impl Plugin for MapGenPlugin
{
    fn build(&self, app: &mut App)
    {
        let map_data = generate_map();
        app.insert_resource(map_data);
    }
}
