use bevy::prelude::*;

use crate::{
    engine::{
        camera::MainCamera,
        coords::GridPos,
        mapgen::MapData,
        prop::{PropType, VariationIndex},
        rendering::macro_map::FollowTransform,
        tile::TileType,
    },
    entity::{spawn_building, spawn_human},
    faction::{FactionId, FactionRegistry},
};

// System set for paint-related systems, used as an ordering anchor.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PaintSet;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ToolMode
{
    #[default]
    Tile,
    Building,
    Human,
}

const TILE_LIST: [TileType; 8] = [
    TileType::Ocean,
    TileType::DeepWater,
    TileType::ShallowWater,
    TileType::Sand,
    TileType::PlainGrass,
    TileType::ForestGrass,
    TileType::Hill,
    TileType::Mountain,
];

const BUILDING_LIST: [PropType; 7] = [
    PropType::HouseTier0,
    PropType::HouseTier1,
    PropType::HouseTier2,
    PropType::HouseTier3,
    PropType::HouseTier4,
    PropType::HouseTier5,
    PropType::HouseTier6,
];

const HUMAN_LIST: [PropType; 4] = [
    PropType::HumanImperialWalking,
    PropType::HumanForestWalking,
    PropType::HumanNorthernWalking,
    PropType::HumanTribalWalking,
];

#[derive(Resource)]
pub struct ToolState
{
    pub mode: ToolMode,
    pub item_index: usize,
    // 0 = no faction / 1..N = index in 0..N-1
    pub faction_index: usize,
    pub eraser_active: bool,
}

impl Default for ToolState
{
    fn default() -> Self
    {
        return Self {
            mode: ToolMode::Tile,
            item_index: 0,
            faction_index: 0,
            eraser_active: false,
        };
    }
}

impl ToolState
{
    fn list_len(&self) -> usize
    {
        return match self.mode
        {
            ToolMode::Tile => TILE_LIST.len(),
            ToolMode::Building => BUILDING_LIST.len(),
            ToolMode::Human => HUMAN_LIST.len(),
        };
    }

    pub fn selected_tile(&self) -> TileType
    {
        return TILE_LIST[self.item_index.min(TILE_LIST.len() - 1)];
    }

    pub fn selected_building(&self) -> PropType
    {
        return BUILDING_LIST[self.item_index.min(BUILDING_LIST.len() - 1)];
    }

    pub fn selected_human(&self) -> PropType
    {
        return HUMAN_LIST[self.item_index.min(HUMAN_LIST.len() - 1)];
    }

    pub fn selected_faction(&self, active_ids: &[FactionId]) -> Option<FactionId>
    {
        if self.faction_index == 0 || active_ids.is_empty()
        {
            return None;
        }
        let idx = (self.faction_index - 1) % active_ids.len();
        return Some(active_ids[idx]);
    }
}

fn handle_tool_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut tool: ResMut<ToolState>,
    registry: Res<FactionRegistry>,
)
{
    if keyboard.just_pressed(KeyCode::Numpad1)
    {
        tool.mode = ToolMode::Tile;
        tool.item_index = 0;
        tool.eraser_active = false;
    }
    if keyboard.just_pressed(KeyCode::Numpad2)
    {
        tool.mode = ToolMode::Building;
        tool.item_index = 0;
        tool.eraser_active = false;
    }
    if keyboard.just_pressed(KeyCode::Numpad3)
    {
        tool.mode = ToolMode::Human;
        tool.item_index = 0;
        tool.eraser_active = false;
    }

    let list_len = tool.list_len();
    if keyboard.just_pressed(KeyCode::Numpad4)
    {
        tool.item_index = if tool.item_index == 0 { list_len - 1 } else { tool.item_index - 1 };
    }
    if keyboard.just_pressed(KeyCode::Numpad6)
    {
        tool.item_index = (tool.item_index + 1) % list_len;
    }

    if keyboard.just_pressed(KeyCode::Numpad0)
    {
        let valid_count = registry.active_ids().len();
        let cycle_len = 1 + valid_count;
        tool.faction_index = (tool.faction_index + 1) % cycle_len.max(1);
    }

    if keyboard.just_pressed(KeyCode::Numpad8)
    {
        tool.eraser_active = !tool.eraser_active;
    }
}

fn cursor_grid_pos(
    windows: &Query<&Window>,
    camera_query: &Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    map_data: &MapData,
) -> Option<(u32, u32)>
{
    let window = windows.single().ok()?;
    let cursor_pos = window.cursor_position()?;
    let (camera, cam_transform) = camera_query.single().ok()?;
    let world_pos = camera
        .viewport_to_world_2d(cam_transform, cursor_pos)
        .ok()?;

    let grid = map_data.world_to_grid(world_pos);
    let map_w = map_data.width_tiles() as i32;
    let map_h = map_data.height_tiles() as i32;

    if grid.x <= 0 || grid.y <= 0 || grid.x >= map_w - 1 || grid.y >= map_h - 1
    {
        return None;
    }

    return Some((grid.x as u32, grid.y as u32));
}

fn use_tool(
    mouse_button: Res<ButtonInput<MouseButton>>,
    tool: Res<ToolState>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut map_data: ResMut<MapData>,
    mut commands: Commands,
    registry: Res<FactionRegistry>,
    prop_entities: Query<(Entity, &GridPos), With<VariationIndex>>,
    dot_entities: Query<(Entity, &FollowTransform)>,
)
{
    if !mouse_button.pressed(MouseButton::Left)
    {
        return;
    }

    let Some((x, y)) = cursor_grid_pos(&windows, &camera_query, &map_data)
    else
    {
        return;
    };

    let grid = GridPos::new(x as i32, y as i32);

    if tool.eraser_active
    {
        despawn_props_at(grid, &prop_entities, &dot_entities, &mut commands);
        return;
    }

    match tool.mode
    {
        ToolMode::Tile =>
        {
            map_data.set_tile(x, y, tool.selected_tile());
        },
        ToolMode::Building =>
        {
            if !mouse_button.just_pressed(MouseButton::Left)
            {
                return;
            }
            let active_ids = registry.active_ids();
            let faction = tool.selected_faction(&active_ids);
            spawn_building(&mut commands, tool.selected_building(), grid, 0, faction);
        },
        ToolMode::Human =>
        {
            if !mouse_button.just_pressed(MouseButton::Left)
            {
                return;
            }
            let active_ids = registry.active_ids();
            let faction = tool.selected_faction(&active_ids);
            spawn_human(&mut commands, tool.selected_human(), grid, faction);
        },
    }
}

fn despawn_props_at(
    target: GridPos,
    prop_entities: &Query<(Entity, &GridPos), With<crate::engine::prop::VariationIndex>>,
    dot_entities: &Query<(Entity, &FollowTransform)>,
    commands: &mut Commands,
)
{
    for (entity, pos) in prop_entities
    {
        if **pos == *target
        {
            commands.entity(entity).despawn();

            for (dot_entity, follow) in dot_entities
            {
                if follow.0 == entity
                {
                    commands.entity(dot_entity).despawn();
                }
            }
        }
    }
}

pub struct PaintingPlugin;

impl Plugin for PaintingPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<ToolState>()
            .add_systems(Update, (handle_tool_input, use_tool).chain().in_set(PaintSet));
    }
}
