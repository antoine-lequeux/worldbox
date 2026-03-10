use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

pub fn setup_chunked_map(mut commands: Commands, asset_server: Res<AssetServer>)
{
    let texture_handle: Handle<Image> = asset_server.load("art/sprites/tileset.png");

    let map_size = TilemapSize { x: 2000, y: 1500 };
    let tile_size = TilemapTileSize { x: 8.0, y: 8.0 };
    let grid_size: TilemapGridSize = tile_size.into();
    let map_type = TilemapType::Square;

    let tilemap_entity = commands.spawn_empty().id();
    let mut tile_storage = TileStorage::empty(map_size);

    for x in 0 .. map_size.x
    {
        for y in 0 .. map_size.y
        {
            let tile_pos = TilePos { x, y };
            let tile_index = 0;

            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture_index: TileTextureIndex(tile_index),
                    ..Default::default()
                })
                .id();

            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        anchor: TilemapAnchor::Center,
        render_settings: TilemapRenderSettings {
            render_chunk_size: UVec2::new(128, 128),
            ..Default::default()
        },
        ..Default::default()
    });
}
