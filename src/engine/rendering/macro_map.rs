use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::tilemap::{ChunkCoord, StandardRenderLayer};
use crate::engine::{
    MACRO_MAP_ZOOM_THRESHOLD,
    coords::GridPos,
    mapgen::MapData,
    painting::PaintSet,
    prop::{AnimationState, PropRegistry, PropType},
    tile::TileRegistry,
};

// Component for entities that appear as a single colored pixel on the macro map.
#[derive(Component)]
pub struct MacroMapDot
{
    // RGBA color drawn on the macro map at this entity's grid position.
    pub color: [u8; 4],
}

// Holds the texture handle for the macro dots.
#[derive(Resource)]
pub struct MacroDotTexture(pub Handle<Image>);

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
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
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
    prop_registry: Res<PropRegistry>,
    prop_query: Query<(&GridPos, &PropType, Option<&AnimationState>)>,
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
            fill_macro_chunk_pixels(
                &mut pixels,
                cx,
                cy,
                &map_data,
                &tile_registry,
                &prop_registry,
                &prop_query,
            );

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

    // Generate a 32x32 anti-aliased circle texture for macro dots.
    let dot_size = 32;
    let mut dot_pixels = vec![0u8; dot_size * dot_size * 4];
    let center = dot_size as f32 / 2.0;
    let radius = center - 1.0;
    for y in 0 .. dot_size
    {
        for x in 0 .. dot_size
        {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = (1.0 - (dist - radius)).clamp(0.0, 1.0) * 255.0;
            let idx = (y * dot_size + x) * 4;
            dot_pixels[idx] = 255;
            dot_pixels[idx + 1] = 255;
            dot_pixels[idx + 2] = 255;
            dot_pixels[idx + 3] = alpha as u8;
        }
    }
    let dot_image = Image::new(
        Extent3d { width: dot_size as u32, height: dot_size as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        dot_pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let dot_texture = images.add(dot_image);
    commands.insert_resource(MacroDotTexture(dot_texture));

    commands.insert_resource(MacroChunkData { chunk_handles });
}

// Composite base tiles and fixed props into a macro chunk's pixel buffer.
fn fill_macro_chunk_pixels(
    pixels: &mut [u8],
    cx: u32,
    cy: u32,
    map_data: &MapData,
    tile_registry: &TileRegistry,
    prop_registry: &PropRegistry,
    prop_query: &Query<(&GridPos, &PropType, Option<&AnimationState>)>,
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

    // Draw fixed props.
    for (pos, prop_type, anim) in prop_query
    {
        let frame = anim.map(|a| a.current_frame).unwrap_or(0);
        if let Some((size, colors)) = prop_registry.get_prop_data(*prop_type, frame)
        {
            let mut i = 0;
            for dy in 0 .. size.y
            {
                for dx in 0 .. size.x
                {
                    let px = pos.x + dx;
                    let py = pos.y + (size.y - 1 - dy);

                    // Check if this pixel falls inside this chunk.
                    if px >= tile_x0 as i32
                        && px < (tile_x0 + cs) as i32
                        && py >= tile_y0 as i32
                        && py < (tile_y0 + cs) as i32
                    {
                        let lx = (px - tile_x0 as i32) as u32;
                        let ly = (py - tile_y0 as i32) as u32;
                        let idx = (ly * cs + lx) as usize * 4;
                        pixels[idx .. idx + 4].copy_from_slice(&colors[i]);
                    }
                    i += 1;
                }
            }
        }
    }
}

// Rebuild dirty macro chunks.
fn update_tile_cache(
    mut map_data: ResMut<MapData>,
    macro_data: Res<MacroChunkData>,
    tile_registry: Res<TileRegistry>,
    prop_registry: Res<PropRegistry>,
    mut images: ResMut<Assets<Image>>,
    prop_query: Query<(&GridPos, &PropType, Option<&AnimationState>)>,
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
                    fill_macro_chunk_pixels(
                        data,
                        cx,
                        cy,
                        &map_data,
                        &tile_registry,
                        &prop_registry,
                        &prop_query,
                    );
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
        *vis = Visibility::Inherited;
    }
}

// Hides the macro map layer.
fn hide_macro(mut q: Query<&mut Visibility, With<MacroRenderLayer>>)
{
    for mut vis in &mut q
    {
        *vis = Visibility::Hidden;
    }
}

// Spawns separate sprite entities for dots so they aren't hidden by their parent's
// StandardRenderLayer.
fn spawn_macro_dots(
    mut commands: Commands,
    query: Query<(Entity, &MacroMapDot), Added<MacroMapDot>>,
    map_data: Res<MapData>,
    dot_texture: Res<MacroDotTexture>,
)
{
    let ts = map_data.tile_size as f32;
    let diameter = ts * (2.0 / 3.0);

    for (entity, dot) in &query
    {
        commands.spawn((
            Sprite {
                image: dot_texture.0.clone(),
                color: Color::srgba_u8(dot.color[0], dot.color[1], dot.color[2], dot.color[3]),
                custom_size: Some(Vec2::splat(diameter)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 1.1),
            MacroRenderLayer,
            FollowTransform(entity),
            Visibility::Hidden,
        ));
    }
}

// Syncs macro dot sprite transforms to their dynamic parents.
fn sync_follow_transforms(
    parent_query: Query<&Transform, Without<FollowTransform>>,
    mut child_query: Query<(&mut Transform, &FollowTransform)>,
)
{
    for (mut tf, follow) in &mut child_query
    {
        if let Ok(parent_tf) = parent_query.get(follow.0)
        {
            tf.translation.x = parent_tf.translation.x;
            tf.translation.y = parent_tf.translation.y;
            // Z=1.1 keeps dots on top of the Z=1.0 macro chunks.
            tf.translation.z = 1.1;
        }
    }
}

// Makes all standard-mode entities visible (except chunks, managed by manage_chunks).
fn show_standard(mut q: Query<&mut Visibility, (With<StandardRenderLayer>, Without<ChunkCoord>)>)
{
    for mut vis in &mut q
    {
        *vis = Visibility::Inherited;
    }
}

// Hides all standard-mode entities.
fn hide_standard(mut q: Query<&mut Visibility, With<StandardRenderLayer>>)
{
    for mut vis in &mut q
    {
        *vis = Visibility::Hidden;
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
            .add_systems(Update, (handle_zoom_states, spawn_macro_dots, sync_follow_transforms))
            .add_systems(OnEnter(MapMode::Macro), (show_macro, hide_standard))
            .add_systems(OnEnter(MapMode::Standard), (show_standard, hide_macro));
    }
}
