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

// State machine controlling which rendering mode is active.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapMode
{
    #[default]
    Standard,
    Macro,
}

const MACRO_GROUP_SIZE: u32 = 10;

// Holds the GPU image handles for macro map chunks.
#[derive(Resource)]
pub struct MacroChunkData
{
    // Indexed by cy * groups_x + cx
    pub group_handles: Vec<Handle<Image>>,
    pub groups_x: u32,
    pub groups_y: u32,
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

    let groups_x = (map_data.chunks_x + MACRO_GROUP_SIZE - 1) / MACRO_GROUP_SIZE;
    let groups_y = (map_data.chunks_y + MACRO_GROUP_SIZE - 1) / MACRO_GROUP_SIZE;

    let mut group_handles = Vec::with_capacity((groups_x * groups_y) as usize);

    for gy in 0 .. groups_y
    {
        for gx in 0 .. groups_x
        {
            let cx_start = gx * MACRO_GROUP_SIZE;
            let cx_end = (cx_start + MACRO_GROUP_SIZE).min(map_data.chunks_x);
            let cy_start = gy * MACRO_GROUP_SIZE;
            let cy_end = (cy_start + MACRO_GROUP_SIZE).min(map_data.chunks_y);

            let w_chunks = cx_end - cx_start;
            let h_chunks = cy_end - cy_start;

            let tex_w = w_chunks * cs;
            let tex_h = h_chunks * cs;

            let mut pixels = vec![0u8; (tex_w * tex_h * 4) as usize];
            fill_macro_group_pixels(
                &mut pixels,
                cx_start,
                cx_end,
                cy_start,
                cy_end,
                tex_w,
                &map_data,
                &tile_registry,
            );

            let image = Image::new(
                Extent3d { width: tex_w, height: tex_h, depth_or_array_layers: 1 },
                TextureDimension::D2,
                pixels,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            let handle = images.add(image);
            group_handles.push(handle.clone());

            let origin_x = map_data.chunk_world_origin(cx_start, cy_start).x;
            let origin_y = map_data.chunk_world_origin(cx_start, cy_start).y;

            let world_w = tex_w as f32 * ts;
            let world_h = tex_h as f32 * ts;

            commands.spawn((
                Sprite {
                    image: handle,
                    custom_size: Some(Vec2::new(world_w, world_h)),
                    ..default()
                },
                Transform::from_xyz(origin_x + world_w / 2.0, origin_y + world_h / 2.0, 1.0)
                    .with_scale(Vec3::new(1.0, -1.0, 1.0)),
                Visibility::Hidden,
                MacroRenderLayer,
            ));
        }
    }

    commands.insert_resource(MacroChunkData { group_handles, groups_x, groups_y });
}

fn fill_macro_group_pixels(
    pixels: &mut [u8],
    cx_start: u32,
    cx_end: u32,
    cy_start: u32,
    cy_end: u32,
    tex_w: u32,
    map_data: &MapData,
    tile_registry: &TileRegistry,
)
{
    let cs = map_data.chunk_size;

    for cy in cy_start .. cy_end
    {
        for cx in cx_start .. cx_end
        {
            let tile_x0 = cx * cs;
            let tile_y0 = cy * cs;

            let local_chunk_x = cx - cx_start;
            let local_chunk_y = cy - cy_start;

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

                    let out_x = local_chunk_x * cs + lx;
                    let out_y = local_chunk_y * cs + ly;
                    let idx = (out_y * tex_w + out_x) as usize * 4;
                    pixels[idx] = color[0];
                    pixels[idx + 1] = color[1];
                    pixels[idx + 2] = color[2];
                    pixels[idx + 3] = 255;
                }
            }
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
    for gy in 0 .. macro_data.groups_y
    {
        for gx in 0 .. macro_data.groups_x
        {
            let cx_start = gx * MACRO_GROUP_SIZE;
            let cx_end = (cx_start + MACRO_GROUP_SIZE).min(map_data.chunks_x);
            let cy_start = gy * MACRO_GROUP_SIZE;
            let cy_end = (cy_start + MACRO_GROUP_SIZE).min(map_data.chunks_y);

            let mut any_dirty = false;
            for cy in cy_start .. cy_end
            {
                for cx in cx_start .. cx_end
                {
                    if map_data.take_macro_chunk_dirty(cx, cy)
                    {
                        any_dirty = true;
                    }
                }
            }

            if any_dirty
            {
                let idx = (gy * macro_data.groups_x + gx) as usize;
                if let Some(image) = images.get_mut(&macro_data.group_handles[idx])
                {
                    if let Some(data) = image.data.as_mut()
                    {
                        let tex_w = (cx_end - cx_start) * map_data.chunk_size;
                        fill_macro_group_pixels(
                            data,
                            cx_start,
                            cx_end,
                            cy_start,
                            cy_end,
                            tex_w,
                            &map_data,
                            &tile_registry,
                        );
                    }
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
