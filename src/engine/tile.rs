use std::collections::HashMap;

use bevy::prelude::*;

// Visual and gameplay properties for a tile type.
#[derive(Clone, Debug)]
pub struct TileDefinition
{
    // Index of the 8x8 template sprite for this tile type in the tileset.
    pub template_idx: usize,
    // Render priority: higher-priority types overlay onto lower-priority tiles.
    pub priority: u8,
    // Color used on the macro (zoomed-out) map.
    pub macro_color: [u8; 3],
}

// Terrain type identifier for each map cell.
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

// Central registry mapping each TileType to its definition.
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
            TileDefinition { template_idx: 0, priority: 4, macro_color: [125, 172, 69] },
        );

        tiles.insert(
            TileType::ForestGrass,
            TileDefinition { template_idx: 1, priority: 5, macro_color: [88, 144, 56] },
        );

        tiles.insert(
            TileType::Sand,
            TileDefinition { template_idx: 2, priority: 3, macro_color: [247, 232, 152] },
        );

        tiles.insert(
            TileType::Hill,
            TileDefinition { template_idx: 3, priority: 6, macro_color: [87, 87, 87] },
        );

        tiles.insert(
            TileType::Mountain,
            TileDefinition { template_idx: 4, priority: 7, macro_color: [61, 61, 61] },
        );

        tiles.insert(
            TileType::ShallowWater,
            TileDefinition { template_idx: 5, priority: 2, macro_color: [92, 181, 240] },
        );

        tiles.insert(
            TileType::DeepWater,
            TileDefinition { template_idx: 6, priority: 1, macro_color: [64, 132, 226] },
        );

        tiles.insert(
            TileType::Ocean,
            TileDefinition { template_idx: 7, priority: 0, macro_color: [51, 112, 204] },
        );

        return Self { tiles };
    }
}
