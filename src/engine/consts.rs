// Size of a single tile in pixels.
pub const TILE_SIZE: u32 = 8;
// Number of tiles per chunk side.
pub const CHUNK_SIZE: u32 = 64;

// Map dimensions in chunks.
pub const MAP_WIDTH: u32 = 110;
pub const MAP_HEIGHT: u32 = 60;

// Seed used by the terrain generator.
pub const MAP_SEED: u64 = 0xab9def1234;
//pub const MAP_SEED: u64 = 0x963258741;
// Number of landmass centers passed to the terrain generator.
pub const LANDMASS_DENSITY: u32 = 12;

// Z layer for prop sprites (above terrain, below UI).
pub const PROP_Z: f32 = 5.0;
// Camera zoom scale above which the macro map is shown.
pub const MACRO_MAP_ZOOM_THRESHOLD: f32 = 1.0;
