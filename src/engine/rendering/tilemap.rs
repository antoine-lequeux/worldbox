use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::autotile;
use crate::engine::{
    MACRO_MAP_ZOOM_THRESHOLD,
    mapgen::MapData,
    painting::PaintSet,
    spritesheet::{AtlasLayoutState, SpritesheetID, SpritesheetRegistry},
    tile::TileRegistry,
};

// Marker component for entities visible only in standard (non-macro) map mode.
#[derive(Component)]
pub struct StandardRenderLayer;

// Identifies which chunk a sprite entity belongs to.
#[derive(Component)]
pub struct ChunkCoord
{
    pub cx: u32,
    pub cy: u32,
}

// CPU-side copy of the tileset image pixels, used for blitting tiles into chunk textures.
#[derive(Resource)]
pub struct TilesetPixels
{
    // Raw RGBA pixel data.
    data: Vec<u8>,
    // Image width in pixels.
    width: u32,
    // Size of one tile in pixels.
    tile_size: u32,
    // Number of tile columns in the tileset.
    columns: u32,
}

impl TilesetPixels
{
    // Extracts pixel data from a loaded tileset image.
    fn from_image(image: &Image, tile_size: u32, columns: u32) -> Self
    {
        return Self {
            data: image
                .data
                .as_ref()
                .expect("tileset image has no pixel data")
                .clone(),
            width: image.width(),
            tile_size,
            columns,
        };
    }
}

// Opaque blit: copies one tile from the tileset into the chunk pixel buffer.
fn blit_tile(
    dst: &mut [u8],
    dst_w: u32,
    dst_x: u32,
    dst_y: u32,
    tileset: &TilesetPixels,
    sprite_idx: u32,
)
{
    let ts = tileset.tile_size;
    let col = sprite_idx % tileset.columns;
    let row = sprite_idx / tileset.columns;
    let src_x0 = col * ts;
    let src_y0 = row * ts;
    let stride = (ts as usize) * 4;

    for sy in 0 .. ts
    {
        let src_off = ((src_y0 + sy) * tileset.width + src_x0) as usize * 4;
        let dst_off = ((dst_y + sy) * dst_w + dst_x) as usize * 4;
        dst[dst_off .. dst_off + stride]
            .copy_from_slice(&tileset.data[src_off .. src_off + stride]);
    }
}

// Alpha-composited blit: draws a tile sprite on top of existing chunk pixels.
fn alpha_blit_tile(
    dst: &mut [u8],
    dst_w: u32,
    dst_x: u32,
    dst_y: u32,
    tileset: &TilesetPixels,
    sprite_idx: u32,
)
{
    let ts = tileset.tile_size;
    let col = sprite_idx % tileset.columns;
    let row = sprite_idx / tileset.columns;
    let src_x0 = col * ts;
    let src_y0 = row * ts;

    for sy in 0 .. ts
    {
        for sx in 0 .. ts
        {
            let si = ((src_y0 + sy) * tileset.width + (src_x0 + sx)) as usize * 4;
            let di = ((dst_y + sy) * dst_w + (dst_x + sx)) as usize * 4;
            let sa = tileset.data[si + 3] as u16;
            if sa == 0
            {
                continue;
            }
            if sa == 255
            {
                dst[di .. di + 4].copy_from_slice(&tileset.data[si .. si + 4]);
            }
            else
            {
                let inv = 255 - sa;
                for c in 0 .. 3
                {
                    dst[di + c] =
                        ((dst[di + c] as u16 * inv + tileset.data[si + c] as u16 * sa) / 255) as u8;
                }
                dst[di + 3] = 255;
            }
        }
    }
}

// Composites all tile base sprites and cardinal overlays into a chunk's pixel buffer.
fn fill_chunk_pixels(
    pixels: &mut [u8],
    cx: u32,
    cy: u32,
    map_data: &MapData,
    tile_registry: &TileRegistry,
    tileset: &TilesetPixels,
)
{
    let cs = map_data.chunk_size;
    let ts = map_data.tile_size;
    let tex_w = cs * ts;
    let tile_x0 = cx * cs;
    let tile_y0 = cy * cs;

    for ly in 0 .. cs
    {
        for lx in 0 .. cs
        {
            let gx = tile_x0 + lx;
            let gy = tile_y0 + ly;
            let tile_type = map_data.get_tile(gx, gy);
            let def = tile_registry.tiles.get(&tile_type).unwrap();

            // Pixel coords: Y is flipped (tile y=0 -> bottom of image).
            let px = lx * ts;
            let py = (cs - 1 - ly) * ts;

            // Base tile (opaque copy).
            blit_tile(pixels, tex_w, px, py, tileset, def.blob_offset);

            // Cardinal overlays (alpha-composited on top).
            for dir in 0 .. 4
            {
                if let Some(overlay_idx) =
                    autotile::compute_overlay_for_dir(gx, gy, dir, map_data, tile_registry)
                {
                    alpha_blit_tile(pixels, tex_w, px, py, tileset, overlay_idx);
                }
            }
        }
    }
}

// Allocates and fills an RGBA Image for a single chunk.
fn build_chunk_image(
    cx: u32,
    cy: u32,
    map_data: &MapData,
    tile_registry: &TileRegistry,
    tileset: &TilesetPixels,
) -> Image
{
    let cs = map_data.chunk_size;
    let ts = map_data.tile_size;
    let tex_w = cs * ts;
    let tex_h = cs * ts;
    let mut pixels = vec![0u8; (tex_w * tex_h * 4) as usize];
    fill_chunk_pixels(&mut pixels, cx, cy, map_data, tile_registry, tileset);

    return Image::new(
        Extent3d { width: tex_w, height: tex_h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
}

// Maximum chunk textures to build per frame after initial load.
const CHUNKS_BUILD_BUDGET: usize = 4;
// Extra chunk margin beyond the viewport for pre-building textures.
const LOAD_MARGIN: f32 = 3.0;
// Chunks beyond this margin have their textures released.
const UNLOAD_MARGIN: f32 = 6.0;

// Tracks chunk texture build state for lazy loading.
#[derive(Resource)]
pub struct ChunkLoadState
{
    // Whether the initial batch of visible chunks has been built.
    initial_load_done: bool,
    // Per-chunk build flag, indexed by cy * chunks_x + cx.
    built: Vec<bool>,
    // Shared placeholder image for unbuilt chunks.
    placeholder: Handle<Image>,
}

// One-time setup: extracts tileset pixels and spawns chunk entities with placeholder images.
fn setup_chunks(
    mut commands: Commands,
    atlas_state: Res<AtlasLayoutState>,
    mut images: ResMut<Assets<Image>>,
    sheet_registry: Res<SpritesheetRegistry>,
    map_data: Res<MapData>,
    mut done: Local<bool>,
)
{
    if *done || !atlas_state.done
    {
        return;
    }
    *done = true;

    // Extract tileset pixel data for CPU blitting.
    let tileset_handle = &sheet_registry.images[&SpritesheetID::Terrain];
    let tileset_image = images.get(tileset_handle).unwrap();
    let tileset_def = sheet_registry.get(SpritesheetID::Terrain).unwrap();
    let tileset = TilesetPixels::from_image(tileset_image, map_data.tile_size, tileset_def.grid.x);

    let num_chunks = (map_data.chunks_x * map_data.chunks_y) as usize;

    // Create a 1x1 transparent placeholder image shared by all unbuilt chunks.
    let placeholder = images.add(Image::new(
        Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        vec![0u8; 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ));

    for cy in 0 .. map_data.chunks_y
    {
        for cx in 0 .. map_data.chunks_x
        {
            let origin = map_data.chunk_world_origin(cx, cy);
            let half = (map_data.chunk_size * map_data.tile_size) as f32 / 2.0;

            commands.spawn((
                Sprite { image: placeholder.clone(), ..default() },
                Transform::from_xyz(origin.x + half, origin.y + half, 0.0),
                ChunkCoord { cx, cy },
                StandardRenderLayer,
                Visibility::Hidden,
            ));
        }
    }

    commands.insert_resource(ChunkLoadState {
        initial_load_done: false,
        built: vec![false; num_chunks],
        placeholder,
    });
    commands.insert_resource(tileset);
}

// Builds textures for nearby chunks and toggles visibility.
fn manage_chunks(
    camera_query: Query<(&Transform, &Projection), With<Camera2d>>,
    windows: Query<&Window>,
    map_data: Res<MapData>,
    tile_registry: Res<TileRegistry>,
    tileset: Res<TilesetPixels>,
    mut load_state: ResMut<ChunkLoadState>,
    mut images: ResMut<Assets<Image>>,
    mut chunk_query: Query<(&ChunkCoord, &mut Visibility, &mut Sprite)>,
)
{
    let Ok((cam_tf, projection)) = camera_query.single()
    else
    {
        return;
    };
    let Projection::Orthographic(ortho) = projection
    else
    {
        return;
    };
    let Ok(window) = windows.single()
    else
    {
        return;
    };

    let in_macro = ortho.scale > MACRO_MAP_ZOOM_THRESHOLD;

    let cam_pos = cam_tf.translation.xy();
    // In macro mode, pretend we are at the zoom threshold so we pre-build
    // the chunks that would be visible right after zooming back in.
    let scale = if in_macro { MACRO_MAP_ZOOM_THRESHOLD } else { ortho.scale };
    let half_w = window.resolution.width() * scale / 2.0;
    let half_h = window.resolution.height() * scale / 2.0;

    let half_map_w = (map_data.width_tiles() * map_data.tile_size) as f32 / 2.0;
    let half_map_h = (map_data.height_tiles() * map_data.tile_size) as f32 / 2.0;
    let chunk_px = (map_data.chunk_size * map_data.tile_size) as f32;

    // Viewport bounds in chunk coordinates.
    let view_min_x = ((cam_pos.x - half_w + half_map_w) / chunk_px)
        .floor()
        .max(0.0) as i32;
    let view_max_x = ((cam_pos.x + half_w + half_map_w) / chunk_px)
        .ceil()
        .min(map_data.chunks_x as f32) as i32;
    let view_min_y = ((cam_pos.y - half_h + half_map_h) / chunk_px)
        .floor()
        .max(0.0) as i32;
    let view_max_y = ((cam_pos.y + half_h + half_map_h) / chunk_px)
        .ceil()
        .min(map_data.chunks_y as f32) as i32;

    // Load range: build textures a few chunks beyond the viewport.
    let load_min_x = (view_min_x as f32 - LOAD_MARGIN).max(0.0) as i32;
    let load_max_x = (view_max_x as f32 + LOAD_MARGIN).min(map_data.chunks_x as f32) as i32;
    let load_min_y = (view_min_y as f32 - LOAD_MARGIN).max(0.0) as i32;
    let load_max_y = (view_max_y as f32 + LOAD_MARGIN).min(map_data.chunks_y as f32) as i32;

    // Unload range: release textures beyond this margin.
    let unload_min_x = (view_min_x as f32 - UNLOAD_MARGIN).max(0.0) as i32;
    let unload_max_x = (view_max_x as f32 + UNLOAD_MARGIN).min(map_data.chunks_x as f32) as i32;
    let unload_min_y = (view_min_y as f32 - UNLOAD_MARGIN).max(0.0) as i32;
    let unload_max_y = (view_max_y as f32 + UNLOAD_MARGIN).min(map_data.chunks_y as f32) as i32;

    // Unlimited budget for the initial load so the first frame isn't blank.
    let budget = if load_state.initial_load_done { CHUNKS_BUILD_BUDGET } else { usize::MAX };
    let mut built_count = 0;

    for (coord, mut vis, mut sprite) in &mut chunk_query
    {
        let cx = coord.cx as i32;
        let cy = coord.cy as i32;
        let idx = (coord.cy * map_data.chunks_x + coord.cx) as usize;

        let in_view = cx >= view_min_x && cx < view_max_x && cy >= view_min_y && cy < view_max_y;
        let in_load = cx >= load_min_x && cx < load_max_x && cy >= load_min_y && cy < load_max_y;
        let in_unload =
            cx >= unload_min_x && cx < unload_max_x && cy >= unload_min_y && cy < unload_max_y;

        // Release textures for chunks that are too far away.
        if !in_unload && load_state.built[idx]
        {
            sprite.image = load_state.placeholder.clone();
            load_state.built[idx] = false;
        }

        // Build textures for chunks in load range.
        if in_load && !load_state.built[idx] && built_count < budget
        {
            let image = build_chunk_image(coord.cx, coord.cy, &map_data, &tile_registry, &tileset);
            sprite.image = images.add(image);
            load_state.built[idx] = true;
            built_count += 1;
        }

        // In macro mode, don't touch visibility (hide_standard owns it).
        if !in_macro
        {
            if in_view && load_state.built[idx]
            {
                *vis = Visibility::Inherited;
            }
            else
            {
                *vis = Visibility::Hidden;
            }
        }
    }

    if !load_state.initial_load_done
    {
        load_state.initial_load_done = true;
    }
}

// Per-frame system: re-composites chunk textures whose tiles have been modified.
fn rebuild_dirty_chunks(
    mut map_data: ResMut<MapData>,
    tile_registry: Res<TileRegistry>,
    tileset: Res<TilesetPixels>,
    load_state: Res<ChunkLoadState>,
    mut images: ResMut<Assets<Image>>,
    chunk_query: Query<(&ChunkCoord, &Sprite)>,
)
{
    // Phase 1: collect dirty chunk coords.
    let mut dirty: Vec<(u32, u32)> = Vec::new();
    for cy in 0 .. map_data.chunks_y
    {
        for cx in 0 .. map_data.chunks_x
        {
            if map_data.take_chunk_dirty(cx, cy)
            {
                dirty.push((cx, cy));
            }
        }
    }

    if dirty.is_empty()
    {
        return;
    }

    // Phase 2: rebuild textures in-place (skip unbuilt chunks).
    for (coord, sprite) in &chunk_query
    {
        if !dirty
            .iter()
            .any(|&(cx, cy)| cx == coord.cx && cy == coord.cy)
        {
            continue;
        }

        // Skip chunks whose textures haven't been built yet.
        let idx = (coord.cy * map_data.chunks_x + coord.cx) as usize;
        if !load_state.built[idx]
        {
            continue;
        }

        if let Some(image) = images.get_mut(&sprite.image)
        {
            if let Some(data) = image.data.as_mut()
            {
                fill_chunk_pixels(data, coord.cx, coord.cy, &map_data, &tile_registry, &tileset);
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
            .add_systems(Update, setup_chunks)
            .add_systems(
                Update,
                manage_chunks
                    .run_if(resource_exists::<TilesetPixels>)
                    .run_if(resource_exists::<ChunkLoadState>),
            )
            .add_systems(
                Update,
                rebuild_dirty_chunks
                    .run_if(resource_exists::<TilesetPixels>)
                    .run_if(resource_exists::<ChunkLoadState>)
                    .after(PaintSet),
            )
            .insert_resource(ClearColor(Color::srgb_u8(48, 104, 187)));
    }
}
