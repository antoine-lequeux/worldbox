use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::engine::{
    consts::{CHUNK_SIZE, MAP_HEIGHT, MAP_WIDTH},
    tile::{TileRegistry, TileType},
};

// Number of cardinal-overlay sprite variants per tile type (2^4 = 16).
// Column 0 = solid fill (base layer). Columns 1-15 = overlay transitions.
pub const CARDINAL_VARIANTS: usize = 16;

// Cardinal bitmask bits. The 4-bit value directly encodes the overlay column index (1-15).
const N: u8 = 1;
const E: u8 = 2;
const S: u8 = 4;
const W: u8 = 8;

// Cardinal neighbor offsets (N, E, S, W) matching the bit order above.
pub const CARDINAL_OFFSETS: [(i32, i32); 4] = [
    (0, 1),  // N
    (1, 0),  // E
    (0, -1), // S
    (-1, 0), // W
];

const CARDINAL_BITS: [u8; 4] = [N, E, S, W];

// Marker for the base terrain tilemap (full fills, rendered first).
#[derive(Component)]
pub struct BaseTilemap;

// Marker for an overlay tilemap. Each overlay layer handles transitions for one tile type.
#[derive(Component)]
pub struct OverlayTilemap
{
    pub overlay_type: TileType,
}

// Which cardinal directions have this specific type as a higher-priority neighbor.
fn analyze_overlay(
    x: u32,
    y: u32,
    center_type: TileType,
    center_priority: u8,
    tile_storage: &TileStorage,
    tile_types: &Query<&TileType>,
    tile_registry: &TileRegistry,
) -> Vec<(TileType, u8)>
{
    let map_w = MAP_WIDTH * CHUNK_SIZE;
    let map_h = MAP_HEIGHT * CHUNK_SIZE;

    let mut per_type: Vec<(TileType, u8)> = Vec::new();

    for (i, &(dx, dy)) in CARDINAL_OFFSETS.iter().enumerate()
    {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;

        if nx < 0 || ny < 0 || nx >= map_w as i32 || ny >= map_h as i32
        {
            continue;
        }

        let neighbor_pos = TilePos { x: nx as u32, y: ny as u32 };
        if let Some(entity) = tile_storage.get(&neighbor_pos)
        {
            if let Ok(neighbor_type) = tile_types.get(entity)
            {
                if *neighbor_type != center_type
                {
                    let priority = tile_registry
                        .tiles
                        .get(neighbor_type)
                        .map(|d| d.priority)
                        .unwrap_or(0);

                    if priority > center_priority
                    {
                        if let Some(entry) = per_type.iter_mut().find(|(t, _)| *t == *neighbor_type)
                        {
                            entry.1 |= CARDINAL_BITS[i];
                        }
                        else
                        {
                            per_type.push((*neighbor_type, CARDINAL_BITS[i]));
                        }
                    }
                }
            }
        }
    }

    return per_type;
}

// System that detects TileType changes on the base layer, collects affected
// positions (changed + 4 cardinal neighbors), and updates all layers.
// Overlay tile entities are spawned on demand and despawned when no longer needed.
pub fn autotile_on_change(
    changed_tiles: Query<&TilePos, Changed<TileType>>,
    tile_registry: Res<TileRegistry>,
    base_storage_query: Query<&TileStorage, (With<BaseTilemap>, Without<OverlayTilemap>)>,
    mut overlay_query: Query<(Entity, &mut TileStorage, &OverlayTilemap), Without<BaseTilemap>>,
    tile_types: Query<&TileType>,
    mut base_textures: Query<&mut TileTextureIndex, With<TileType>>,
    mut overlay_data: Query<&mut TileTextureIndex, Without<TileType>>,
    mut commands: Commands,
)
{
    let Ok(base_storage) = base_storage_query.single()
    else
    {
        return;
    };

    let map_w = (MAP_WIDTH * CHUNK_SIZE) as i32;
    let map_h = (MAP_HEIGHT * CHUNK_SIZE) as i32;

    let mut positions = Vec::new();

    for tile_pos in &changed_tiles
    {
        let x = tile_pos.x as i32;
        let y = tile_pos.y as i32;

        positions.push(UVec2::new(tile_pos.x, tile_pos.y));

        for &(dx, dy) in &CARDINAL_OFFSETS
        {
            let nx = x + dx;
            let ny = y + dy;
            if nx >= 0 && ny >= 0 && nx < map_w && ny < map_h
            {
                positions.push(UVec2::new(nx as u32, ny as u32));
            }
        }
    }

    if positions.is_empty()
    {
        return;
    }

    positions.sort_unstable_by(|a, b| a.x.cmp(&b.x).then(a.y.cmp(&b.y)));
    positions.dedup();

    for pos in &positions
    {
        let tile_pos = TilePos { x: pos.x, y: pos.y };

        // Base layer.
        let Some(base_entity) = base_storage.get(&tile_pos)
        else
        {
            continue;
        };
        let Ok(tile_type) = tile_types.get(base_entity)
        else
        {
            continue;
        };
        let Some(center_def) = tile_registry.tiles.get(tile_type)
        else
        {
            continue;
        };

        let base_index = center_def.blob_offset;
        if let Ok(mut tex) = base_textures.get_mut(base_entity)
        {
            if tex.0 != base_index
            {
                tex.0 = base_index;
            }
        }

        // Overlay layers (one per tile type).
        let analysis = analyze_overlay(
            pos.x,
            pos.y,
            *tile_type,
            center_def.priority,
            base_storage,
            &tile_types,
            &tile_registry,
        );

        for (tilemap_entity, mut storage, marker) in &mut overlay_query
        {
            let overlay_type = marker.overlay_type;
            let existing = storage.get(&tile_pos);

            if let Some(&(_, bitmask)) = analysis.iter().find(|(t, _)| *t == overlay_type)
            {
                let foreign_def = tile_registry.tiles.get(&overlay_type).unwrap();
                let new_index = foreign_def.blob_offset + bitmask as u32;

                match existing
                {
                    Some(entity) =>
                    {
                        if let Ok(mut tex) = overlay_data.get_mut(entity)
                        {
                            if tex.0 != new_index
                            {
                                tex.0 = new_index;
                            }
                        }
                    },
                    None =>
                    {
                        let new_entity = commands
                            .spawn(TileBundle {
                                position: tile_pos,
                                tilemap_id: TilemapId(tilemap_entity),
                                texture_index: TileTextureIndex(new_index),
                                ..Default::default()
                            })
                            .id();
                        storage.set(&tile_pos, new_entity);
                    },
                }
            }
            else if let Some(entity) = existing
            {
                commands.entity(entity).despawn();
                storage.remove(&tile_pos);
            }
        }
    }
}

pub struct AutotilePlugin;

impl Plugin for AutotilePlugin
{
    fn build(&self, app: &mut App)
    {
        app.add_systems(PostUpdate, autotile_on_change);
    }
}
