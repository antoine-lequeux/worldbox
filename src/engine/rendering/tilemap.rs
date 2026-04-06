use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::engine::{
    autotile::{BaseTilemap, OverlayTilemap},
    consts::{CHUNK_SIZE, MAP_HEIGHT, MAP_WIDTH, TILE_SIZE},
    coords::GridPos,
    spritesheet::{SpritesheetID, SpritesheetRegistry},
    tile::{TileRegistry, TileType},
};

#[derive(Component)]
pub struct BorderTile;

// All tile types in priority order (low -> high) for overlay layer creation.
const ALL_TILE_TYPES: [TileType; 8] = [
    TileType::Ocean,
    TileType::DeepWater,
    TileType::ShallowWater,
    TileType::Sand,
    TileType::PlainGrass,
    TileType::ForestGrass,
    TileType::Hill,
    TileType::Mountain,
];

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

    let texture = TilemapTexture::Single(sheet_registry.images[&SpritesheetID::Terrain].clone());

    let render_settings =
        TilemapRenderSettings { render_chunk_size: UVec2::splat(CHUNK_SIZE), ..Default::default() };

    // Base tilemap.
    let base_entity = commands.spawn_empty().id();
    let mut base_storage = TileStorage::empty(map_size);

    for x in 0 .. map_size.x
    {
        for y in 0 .. map_size.y
        {
            let tile_type = TileType::Ocean;
            let is_border = x == 0 || y == 0 || x == map_size.x - 1 || y == map_size.y - 1;

            let def = tile_registry.tiles.get(&tile_type).unwrap();
            let tile_pos = TilePos { x, y };

            let tile_id = commands
                .spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(base_entity),
                        texture_index: TileTextureIndex(def.blob_offset),
                        ..Default::default()
                    },
                    GridPos(IVec2 { x: x as i32, y: y as i32 }),
                    StandardRenderLayer,
                    tile_type,
                ))
                .id();

            if is_border
            {
                // Border tiles are not modifiable (to prevent overlay problems).
                commands.entity(tile_id).insert(BorderTile);
            }

            base_storage.set(&tile_pos, tile_id);
        }
    }

    commands.entity(base_entity).insert((
        TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: base_storage,
            texture: texture.clone(),
            tile_size,
            anchor: TilemapAnchor::Center,
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            render_settings,
            ..Default::default()
        },
        BaseTilemap,
        StandardRenderLayer,
    ));

    // One overlay tilemap per tile type, stacked by priority.
    for &overlay_type in &ALL_TILE_TYPES
    {
        let priority = tile_registry
            .tiles
            .get(&overlay_type)
            .map(|d| d.priority)
            .unwrap_or(0);

        let overlay_entity = commands.spawn_empty().id();
        let overlay_storage = TileStorage::empty(map_size);

        // z = 0.01 * (priority + 1) so overlays stack in priority order above the base.
        let z = 0.01 * (priority as f32 + 1.0);

        commands.entity(overlay_entity).insert((
            TilemapBundle {
                grid_size,
                map_type,
                size: map_size,
                storage: overlay_storage,
                texture: texture.clone(),
                tile_size,
                anchor: TilemapAnchor::Center,
                transform: Transform::from_xyz(0.0, 0.0, z),
                render_settings,
                ..Default::default()
            },
            OverlayTilemap { overlay_type },
            StandardRenderLayer,
        ));
    }
}

pub struct CustomTilemapPlugin;

impl Plugin for CustomTilemapPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<TileRegistry>()
            .add_systems(PostStartup, setup_chunked_map)
            .insert_resource(ClearColor(Color::srgb_u8(48, 104, 187)));
    }
}
