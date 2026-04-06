use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::engine::{
    autotile::BaseTilemap,
    consts::{CHUNK_SIZE, MAP_HEIGHT, MAP_WIDTH},
    coords::world_to_grid,
    rendering::tilemap::BorderTile,
    tile::TileType,
};

const NUMPAD_BINDINGS: [(KeyCode, TileType); 8] = [
    (KeyCode::Numpad1, TileType::Ocean),
    (KeyCode::Numpad2, TileType::DeepWater),
    (KeyCode::Numpad3, TileType::ShallowWater),
    (KeyCode::Numpad4, TileType::Sand),
    (KeyCode::Numpad5, TileType::PlainGrass),
    (KeyCode::Numpad6, TileType::ForestGrass),
    (KeyCode::Numpad7, TileType::Hill),
    (KeyCode::Numpad8, TileType::Mountain),
];

#[derive(Resource)]
pub struct SelectedBrush
{
    pub tile_type: TileType,
}

impl Default for SelectedBrush
{
    fn default() -> Self
    {
        return Self { tile_type: TileType::Sand };
    }
}

fn select_brush(keyboard: Res<ButtonInput<KeyCode>>, mut brush: ResMut<SelectedBrush>)
{
    for &(key, tile_type) in &NUMPAD_BINDINGS
    {
        if keyboard.just_pressed(key)
        {
            brush.tile_type = tile_type;
        }
    }
}

fn paint_tiles(
    mouse_button: Res<ButtonInput<MouseButton>>,
    brush: Res<SelectedBrush>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<crate::engine::camera::MainCamera>>,
    mut base_tilemap_query: Query<&mut TileStorage, With<BaseTilemap>>,
    mut tile_type_query: Query<&mut TileType, Without<BorderTile>>,
)
{
    if !mouse_button.pressed(MouseButton::Left)
    {
        return;
    }

    let Ok(window) = windows.single()
    else
    {
        return;
    };
    let Some(cursor_pos) = window.cursor_position()
    else
    {
        return;
    };
    let Ok((camera, cam_transform)) = camera_query.single()
    else
    {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos)
    else
    {
        return;
    };

    let grid = world_to_grid(world_pos);

    let map_w = (MAP_WIDTH * CHUNK_SIZE) as i32;
    let map_h = (MAP_HEIGHT * CHUNK_SIZE) as i32;
    if grid.x < 0 || grid.y < 0 || grid.x >= map_w || grid.y >= map_h
    {
        return;
    }

    let Ok(storage) = base_tilemap_query.single_mut()
    else
    {
        return;
    };
    let tile_pos = TilePos { x: grid.x as u32, y: grid.y as u32 };

    if let Some(tile_entity) = storage.get(&tile_pos)
    {
        if let Ok(mut tile_type) = tile_type_query.get_mut(tile_entity)
        {
            if *tile_type != brush.tile_type
            {
                *tile_type = brush.tile_type;
            }
        }
    }
}

pub struct PaintingPlugin;

impl Plugin for PaintingPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<SelectedBrush>()
            .add_systems(Update, (select_brush, paint_tiles).chain());
    }
}
