use std::collections::HashMap;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::engine::{
    consts::{CHUNK_SIZE, MAP_HEIGHT, MAP_WIDTH, TILE_SIZE},
    coords::GridPos,
    spritesheet::{SpritesheetID, SpritesheetRegistry},
    tile::{TileRegistry, TileType},
};

#[derive(Component)]
pub struct StandardRenderLayer;

pub fn setup_chunked_map(
    mut commands: Commands,
    sheet_registry: Res<SpritesheetRegistry>,
    tile_registry: Res<TileRegistry>,
)
{
    let map_size = TilemapSize { x: MAP_WIDTH * CHUNK_SIZE, y: MAP_HEIGHT * CHUNK_SIZE };
    let tile_size = TilemapTileSize { x: TILE_SIZE as f32, y: TILE_SIZE as f32 };
    let grid_size: TilemapGridSize = tile_size.into();
    let map_type = TilemapType::Square;

    let tile_sheets = &[SpritesheetID::Terrain /* , SpritesheetID::Terrain2... */];
    let mut tilemaps: HashMap<SpritesheetID, (Entity, TileStorage)> = tile_sheets
        .iter()
        .map(|id| (*id, (commands.spawn_empty().id(), TileStorage::empty(map_size))))
        .collect();

    for x in 0 .. map_size.x
    {
        for y in 0 .. map_size.y
        {
            let tile_type = match (x + y) % 4
            {
                0 => TileType::Grass,
                1 => TileType::Rock,
                2 => TileType::Dirt,
                _ => TileType::Water,
            };

            let def = tile_registry.tiles.get(&tile_type).unwrap();
            let sheet = sheet_registry.get(def.sheet_id).unwrap();
            let sprite_index = sheet.sprite_index(def.sprite.x, def.sprite.y);

            let (tilemap_entity, tile_storage) = tilemaps.get_mut(&def.sheet_id).unwrap();
            let tile_pos = TilePos { x, y };

            let tile_entity = commands
                .spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(*tilemap_entity),
                        texture_index: TileTextureIndex(sprite_index),
                        ..Default::default()
                    },
                    GridPos(IVec2 { x: x as i32, y: y as i32 }),
                    StandardRenderLayer,
                    tile_type,
                ))
                .id();

            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    for (i, sheet_id) in tile_sheets.iter().enumerate()
    {
        let (tilemap_entity, tile_storage) = tilemaps.remove(sheet_id).unwrap();
        let texture = TilemapTexture::Single(sheet_registry.images[sheet_id].clone());

        commands.entity(tilemap_entity).insert(TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: tile_storage,
            texture,
            tile_size,
            anchor: TilemapAnchor::Center,
            transform: Transform::from_xyz(0.0, 0.0, i as f32 * 0.1),
            render_settings: TilemapRenderSettings {
                render_chunk_size: UVec2::splat(CHUNK_SIZE),
                ..Default::default()
            },
            ..Default::default()
        });
    }
}

fn sync_tile_visuals(
    tile_registry: Res<TileRegistry>,
    sheet_registry: Res<SpritesheetRegistry>,
    mut tile_query: Query<(&TileType, &mut TileTextureIndex), Changed<TileType>>,
)
{
    for (tile_type, mut texture_index) in &mut tile_query
    {
        if let Some(def) = tile_registry.tiles.get(tile_type)
        {
            if let Some(sheet) = sheet_registry.get(def.sheet_id)
            {
                texture_index.0 = sheet.sprite_index(def.sprite.x, def.sprite.y);
            }
        }
    }
}

pub struct CustomTilemapPlugin;

impl Plugin for CustomTilemapPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<TileRegistry>()
            .add_systems(PostStartup, setup_chunked_map)
            .add_systems(Update, sync_tile_visuals);
    }
}
