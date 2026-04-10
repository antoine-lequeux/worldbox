use bevy::prelude::*;

use crate::engine::{mapgen::MapData, tile::TileType};

// System set for paint-related systems, used as an ordering anchor.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PaintSet;

// Numpad key bindings mapping keys to tile types.
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

// Tracks the currently selected tile type for painting.
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

// Switches the active brush when a numpad key is pressed.
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

// Paints tiles under the cursor while the left mouse button is held.
fn paint_tiles(
    mouse_button: Res<ButtonInput<MouseButton>>,
    brush: Res<SelectedBrush>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<crate::engine::camera::MainCamera>>,
    mut map_data: ResMut<MapData>,
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

    let grid = map_data.world_to_grid(world_pos);

    let map_w = map_data.width_tiles() as i32;
    let map_h = map_data.height_tiles() as i32;
    if grid.x < 0 || grid.y < 0 || grid.x >= map_w || grid.y >= map_h
    {
        return;
    }

    // Border tiles (edge of map) are not paintable.
    let x = grid.x as u32;
    let y = grid.y as u32;
    if x == 0 || y == 0 || x == map_data.width_tiles() - 1 || y == map_data.height_tiles() - 1
    {
        return;
    }

    map_data.set_tile(x, y, brush.tile_type);
}

pub struct PaintingPlugin;

impl Plugin for PaintingPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<SelectedBrush>()
            .add_systems(Update, (select_brush, paint_tiles).chain().in_set(PaintSet));
    }
}
