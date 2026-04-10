use std::collections::HashMap;

use bevy::prelude::*;

// Visual and gameplay properties for a tile type.
#[derive(Clone, Debug)]
pub struct TileDefinition
{
    // Starting sprite index in the spritesheet for this type's 16-tile cardinal set.
    pub blob_offset: u32,
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
            TileDefinition { blob_offset: 0, priority: 4, macro_color: [125, 172, 69] },
        );

        tiles.insert(
            TileType::ForestGrass,
            TileDefinition { blob_offset: 16, priority: 5, macro_color: [106, 145, 64] },
        );

        tiles.insert(
            TileType::Sand,
            TileDefinition { blob_offset: 32, priority: 3, macro_color: [222, 216, 151] },
        );

        tiles.insert(
            TileType::Hill,
            TileDefinition { blob_offset: 48, priority: 6, macro_color: [101, 99, 90] },
        );

        tiles.insert(
            TileType::Mountain,
            TileDefinition { blob_offset: 64, priority: 7, macro_color: [70, 69, 64] },
        );

        tiles.insert(
            TileType::ShallowWater,
            TileDefinition { blob_offset: 80, priority: 2, macro_color: [84, 170, 231] },
        );

        tiles.insert(
            TileType::DeepWater,
            TileDefinition { blob_offset: 96, priority: 1, macro_color: [67, 138, 235] },
        );

        tiles.insert(
            TileType::Ocean,
            TileDefinition { blob_offset: 112, priority: 0, macro_color: [48, 104, 187] },
        );

        return Self { tiles };
    }
}
