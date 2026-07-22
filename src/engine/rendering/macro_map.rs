use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::tilemap::{ChunkCoord, StandardRenderLayer};
use crate::engine::{
    MACRO_MAP_ZOOM_THRESHOLD, mapgen::MapData, painting::PaintSet, tile::TileRegistry,
};

// Component for dynamic entities on the macro map.
#[derive(Component)]
pub struct MacroMapEntity
{
    pub color: [u8; 4],
}

// Marker for an entity that syncs its transform to another entity.
#[derive(Component)]
pub struct FollowTransform(pub Entity);

// Marker component for entities visible only in macro map mode.
#[derive(Component)]
pub struct MacroRenderLayer;

// Identifies which chunk a macro map sprite belongs to.
#[derive(Component)]
pub struct MacroMapChunk
{
    pub cx: u32,
    pub cy: u32,
}

// State machine controlling which rendering mode is active.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapMode
{
    #[default]
    Standard,
    Macro,
}

// Holds the GPU image handles for macro map chunks.
#[derive(Resource)]
pub struct MacroChunkData
{
    // Indexed by cy * chunks_x + cx
    pub chunk_handles: Vec<Handle<Image>>,
}

// Initializes the macro map: spawns chunk entities with placeholder textures.
fn init_macro_engine(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    map_data: Res<MapData>,
    tile_registry: Res<TileRegistry>,
)
{
    let cs = map_data.chunk_size;
    let ts = map_data.tile_size as f32;
    let chunk_world_size = cs as f32 * ts;
    let mut chunk_handles = Vec::with_capacity((map_data.chunks_x * map_data.chunks_y) as usize);

    for cy in 0 .. map_data.chunks_y
    {
        for cx in 0 .. map_data.chunks_x
        {
            let mut pixels = vec![0u8; (cs * cs * 4) as usize];
            fill_macro_chunk_pixels(&mut pixels, cx, cy, &map_data, &tile_registry);

            let image = Image::new(
                Extent3d { width: cs, height: cs, depth_or_array_layers: 1 },
                TextureDimension::D2,
                pixels,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            let handle = images.add(image);
            chunk_handles.push(handle.clone());

            let origin = map_data.chunk_world_origin(cx, cy);
            let half = chunk_world_size / 2.0;

            commands.spawn((
                Sprite {
                    image: handle,
                    custom_size: Some(Vec2::splat(chunk_world_size)),
                    ..default()
                },
                Transform::from_xyz(origin.x + half, origin.y + half, 1.0)
                    .with_scale(Vec3::new(1.0, -1.0, 1.0)),
                Visibility::Hidden,
                MacroMapChunk { cx, cy },
                MacroRenderLayer,
            ));
        }
    }

    commands.insert_resource(MacroChunkData { chunk_handles });
}

fn fill_macro_chunk_pixels(
    pixels: &mut [u8],
    cx: u32,
    cy: u32,
    map_data: &MapData,
    tile_registry: &TileRegistry,
)
{
    let cs = map_data.chunk_size;
    let tile_x0 = cx * cs;
    let tile_y0 = cy * cs;

    // Draw base tiles.
    for ly in 0 .. cs
    {
        for lx in 0 .. cs
        {
            let gx = tile_x0 + lx;
            let gy = tile_y0 + ly;
            let tile_type = map_data.get_tile(gx, gy);
            let color = tile_registry
                .tiles
                .get(&tile_type)
                .map(|d| d.macro_color)
                .unwrap_or([0, 0, 0]);

            let idx = (ly * cs + lx) as usize * 4;
            pixels[idx] = color[0];
            pixels[idx + 1] = color[1];
            pixels[idx + 2] = color[2];
            pixels[idx + 3] = 255;
        }
    }
}

fn update_tile_cache(
    mut map_data: ResMut<MapData>,
    macro_data: Res<MacroChunkData>,
    tile_registry: Res<TileRegistry>,
    mut images: ResMut<Assets<Image>>,
)
{
    for cy in 0 .. map_data.chunks_y
    {
        for cx in 0 .. map_data.chunks_x
        {
            if !map_data.take_macro_chunk_dirty(cx, cy)
            {
                continue;
            }

            let idx = (cy * map_data.chunks_x + cx) as usize;
            if let Some(image) = images.get_mut(&macro_data.chunk_handles[idx])
            {
                if let Some(data) = image.data.as_mut()
                {
                    fill_macro_chunk_pixels(data, cx, cy, &map_data, &tile_registry);
                }
            }
        }
    }
}

// Switches between Standard and Macro mode based on camera zoom level.
fn handle_zoom_states(
    camera_query: Query<&Projection, (With<Camera2d>, Changed<Projection>)>,
    current_state: Res<State<MapMode>>,
    mut next_state: ResMut<NextState<MapMode>>,
)
{
    if let Ok(Projection::Orthographic(ortho)) = camera_query.single()
    {
        if *current_state.get() == MapMode::Standard && ortho.scale > MACRO_MAP_ZOOM_THRESHOLD
        {
            next_state.set(MapMode::Macro);
        }
        else if *current_state.get() == MapMode::Macro && ortho.scale <= MACRO_MAP_ZOOM_THRESHOLD
        {
            next_state.set(MapMode::Standard);
        }
    }
}

// Shows the macro map layer.
fn show_macro(mut q: Query<&mut Visibility, With<MacroRenderLayer>>)
{
    for mut vis in &mut q
    {
        if *vis != Visibility::Inherited
        {
            *vis = Visibility::Inherited;
        }
    }
}

// Hides the macro map layer.
fn hide_macro(mut q: Query<&mut Visibility, With<MacroRenderLayer>>)
{
    for mut vis in &mut q
    {
        if *vis != Visibility::Hidden
        {
            *vis = Visibility::Hidden;
        }
    }
}

// Spawns separate sprite entities for dynamic objects.
fn spawn_macro_entities(
    mut commands: Commands,
    query: Query<(Entity, &MacroMapEntity), Added<MacroMapEntity>>,
    map_data: Res<MapData>,
    map_mode: Res<State<MapMode>>,
)
{
    let ts = map_data.tile_size as f32;
    let initial_vis =
        if *map_mode.get() == MapMode::Macro { Visibility::Inherited } else { Visibility::Hidden };

    for (entity, dot) in &query
    {
        commands.spawn((
            Sprite {
                color: Color::srgba_u8(dot.color[0], dot.color[1], dot.color[2], dot.color[3]),
                custom_size: Some(Vec2::splat(ts)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 1.1),
            MacroRenderLayer,
            FollowTransform(entity),
            initial_vis,
        ));
    }
}

// Syncs macro sprite transforms to their dynamic parents, snapping to the grid.
fn sync_follow_transforms(
    map_mode: Res<State<MapMode>>,
    map_data: Res<MapData>,
    time: Res<Time>,
    parent_query: Query<&Transform, Without<FollowTransform>>,
    mut child_query: Query<(&mut Transform, &FollowTransform)>,
)
{
    if *map_mode.get() != MapMode::Macro
    {
        return;
    }

    let ts = map_data.tile_size as f32;
    // We track a current Z to apply to entities every time they enter a new tile.
    // Current Z is slowly incremented so that the last entity to enter a tile is displayed on top.
    let current_z = 1.1 + (time.elapsed_secs() % 10000.0) * 0.00001;

    for (mut tf, follow) in &mut child_query
    {
        if let Ok(parent_tf) = parent_query.get(follow.0)
        {
            let p_x = parent_tf.translation.x;
            let p_y = parent_tf.translation.y;

            let grid_x = (p_x / ts).floor();
            let grid_y = (p_y / ts).floor();

            let new_x = grid_x * ts + ts / 2.0;
            let new_y = grid_y * ts + ts / 2.0;

            if tf.translation.x != new_x || tf.translation.y != new_y
            {
                tf.translation.x = new_x;
                tf.translation.y = new_y;
                tf.translation.z = current_z;
            }
        }
    }
}

// Makes all standard-mode entities visible (except chunks, managed by manage_chunks).
fn show_standard(mut q: Query<&mut Visibility, (With<StandardRenderLayer>, Without<ChunkCoord>)>)
{
    for mut vis in &mut q
    {
        if *vis != Visibility::Inherited
        {
            *vis = Visibility::Inherited;
        }
    }
}

// Hides all standard-mode entities.
fn hide_standard(mut q: Query<&mut Visibility, With<StandardRenderLayer>>)
{
    for mut vis in &mut q
    {
        if *vis != Visibility::Hidden
        {
            *vis = Visibility::Hidden;
        }
    }
}

pub struct MacroMapPlugin;

impl Plugin for MacroMapPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_state::<MapMode>()
            .add_systems(
                Startup,
                init_macro_engine.after(crate::engine::spritesheet::build_atlas_layouts),
            )
            .add_systems(Update, update_tile_cache.after(PaintSet))
            .add_systems(Update, (handle_zoom_states, spawn_macro_entities, sync_follow_transforms))
            .add_systems(OnEnter(MapMode::Macro), (show_macro, hide_standard))
            .add_systems(OnEnter(MapMode::Standard), (show_standard, hide_macro));
    }
}
