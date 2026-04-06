use std::collections::HashMap;

use bevy::prelude::*;

use crate::engine::{autotile::CARDINAL_VARIANTS, spritesheet::SpritesheetID};

// Create a macro_colors array where all 16 sprites share the same color.
pub fn uniform_macro_colors(color: [u8; 3]) -> Vec<[u8; 3]>
{
    return vec![color; CARDINAL_VARIANTS];
}

#[derive(Clone, Debug)]
pub struct TileDefinition
{
    pub sheet_id: SpritesheetID,
    // Starting sprite index in the spritesheet for this type's 16-tile cardinal set.
    pub blob_offset: u32,
    // Render priority: higher-priority types overlay onto lower-priority tiles.
    pub priority: u8,
    // Per-variant macro map colors (length must be CARDINAL_VARIANTS = 16).
    // Index 0 = no neighbors, index 15 = all 4 cardinal neighbors (full fill).
    pub macro_colors: Vec<[u8; 3]>,
}

impl TileDefinition
{
    // Get the macro color for a specific variant index (0..15).
    pub fn macro_color(&self, blob_index: usize) -> [u8; 3]
    {
        return self
            .macro_colors
            .get(blob_index)
            .copied()
            .unwrap_or(self.macro_colors.last().copied().unwrap_or([0, 0, 0]));
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TileType
{
    PlainGrass,
    ForestGrass,
    Sand,
    Hill,
    Mountain,
    ShallowWater,
    DeepWater,
    Ocean,
}

#[derive(Resource)]
pub struct TileRegistry
{
    pub tiles: HashMap<TileType, TileDefinition>,
}

impl Default for TileRegistry
{
    fn default() -> Self
    {
        let mut tiles = HashMap::new();

        tiles.insert(
            TileType::PlainGrass,
            TileDefinition {
                sheet_id: SpritesheetID::Terrain,
                blob_offset: 0,
                priority: 4,
                macro_colors: uniform_macro_colors([125, 172, 69]),
            },
        );

        tiles.insert(
            TileType::ForestGrass,
            TileDefinition {
                sheet_id: SpritesheetID::Terrain,
                blob_offset: 16,
                priority: 5,
                macro_colors: uniform_macro_colors([106, 145, 64]),
            },
        );

        tiles.insert(
            TileType::Sand,
            TileDefinition {
                sheet_id: SpritesheetID::Terrain,
                blob_offset: 32,
                priority: 3,
                macro_colors: uniform_macro_colors([222, 216, 151]),
            },
        );

        tiles.insert(
            TileType::Hill,
            TileDefinition {
                sheet_id: SpritesheetID::Terrain,
                blob_offset: 48,
                priority: 6,
                macro_colors: uniform_macro_colors([101, 99, 90]),
            },
        );

        tiles.insert(
            TileType::Mountain,
            TileDefinition {
                sheet_id: SpritesheetID::Terrain,
                blob_offset: 64,
                priority: 7,
                macro_colors: uniform_macro_colors([70, 69, 64]),
            },
        );

        tiles.insert(
            TileType::ShallowWater,
            TileDefinition {
                sheet_id: SpritesheetID::Terrain,
                blob_offset: 80,
                priority: 2,
                macro_colors: uniform_macro_colors([84, 170, 231]),
            },
        );

        tiles.insert(
            TileType::DeepWater,
            TileDefinition {
                sheet_id: SpritesheetID::Terrain,
                blob_offset: 96,
                priority: 1,
                macro_colors: uniform_macro_colors([67, 138, 235]),
            },
        );

        tiles.insert(
            TileType::Ocean,
            TileDefinition {
                sheet_id: SpritesheetID::Terrain,
                blob_offset: 112,
                priority: 0,
                macro_colors: uniform_macro_colors([48, 104, 187]),
            },
        );

        return Self { tiles };
    }
}

impl TileRegistry
{
    // Get the macro color for a tile, given its type and current blob sprite index.
    pub fn get_macro_color(&self, tile_type: TileType, texture_index: u32) -> [u8; 3]
    {
        if let Some(def) = self.tiles.get(&tile_type)
        {
            let blob_index = texture_index.saturating_sub(def.blob_offset) as usize;
            return def.macro_color(blob_index);
        }
        return [0, 0, 0];
    }
}
