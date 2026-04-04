use std::collections::HashMap;

use bevy::prelude::*;

use crate::engine::spritesheet::SpritesheetID;

#[derive(Clone, Debug)]
pub struct TileDefinition
{
    pub macro_color: [u8; 3],
    pub sheet_id: SpritesheetID,
    // Column (x) and row (y) of the sprite within the sheet.
    pub sprite: UVec2,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TileType
{
    Grass,
    Rock,
    Dirt,
    Water,
}

#[derive(Resource)]
pub struct TileRegistry
{
    pub tiles: HashMap<TileType, TileDefinition>,
}

impl Default for TileRegistry
{
    // All tiles are registered here.
    fn default() -> Self
    {
        let mut tiles = HashMap::new();

        tiles.insert(
            TileType::Grass,
            TileDefinition {
                macro_color: [140, 215, 67],
                sheet_id: SpritesheetID::Terrain,
                sprite: UVec2::new(0, 0),
            },
        );
        tiles.insert(
            TileType::Rock,
            TileDefinition {
                macro_color: [89, 86, 82],
                sheet_id: SpritesheetID::Terrain,
                sprite: UVec2::new(1, 0),
            },
        );
        tiles.insert(
            TileType::Dirt,
            TileDefinition {
                macro_color: [143, 86, 59],
                sheet_id: SpritesheetID::Terrain,
                sprite: UVec2::new(0, 1),
            },
        );
        tiles.insert(
            TileType::Water,
            TileDefinition {
                macro_color: [99, 155, 255],
                sheet_id: SpritesheetID::Terrain,
                sprite: UVec2::new(1, 1),
            },
        );

        return Self { tiles };
    }
}

impl TileRegistry
{
    pub fn get_color(&self, tile_type: TileType) -> [u8; 3]
    {
        return self
            .tiles
            .get(&tile_type)
            .map(|d| d.macro_color)
            .unwrap_or([0, 0, 0]);
    }
}
