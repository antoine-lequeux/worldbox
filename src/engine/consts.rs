// Size of a single tile in pixels.
pub const TILE_SIZE: u32 = 8;
// Number of tiles per chunk side.
pub const CHUNK_SIZE: u32 = 64;

// Map dimensions in chunks.
pub const MAP_WIDTH: u32 = 10;
pub const MAP_HEIGHT: u32 = 10;

// Seed used by the Python terrain generator.
pub const MAP_SEED: u64 = 11072023;
// Number of continents passed to the terrain generator.
pub const NUM_CONTINENTS: u32 = 3;

// Z layer for prop sprites (above terrain, below UI).
pub const PROP_Z: f32 = 5.0;
// Camera zoom scale above which the macro map is shown.
pub const MACRO_MAP_ZOOM_THRESHOLD: f32 = 1.7;
