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
pub struct ParsedTemplate
{
    pub base: [u8; 64],
    pub overlay_from_n: [u8; 64],
    pub overlay_from_e: [u8; 64],
    pub overlay_from_s: [u8; 64],
    pub overlay_from_w: [u8; 64],
}

// CPU-side copy of the tileset image pixels, used for blitting tiles into chunk textures.
#[derive(Resource)]
pub struct TilesetPixels
{
    pub templates: Vec<Vec<ParsedTemplate>>,
    // Size of one tile in pixels.
    pub tile_size: u32,
}

impl TilesetPixels
{
    // Extracts pixel data from a loaded tileset image.
    fn from_image(image: &Image, tile_size: u32, columns: u32) -> Self
    {
        let data = image
            .data
            .as_ref()
            .expect("tileset image has no pixel data");
        let width = image.width();
        let rows = image.height() / tile_size;
        let mut templates = Vec::new();

        for r in 0 .. rows
        {
            let mut row_variations = Vec::new();
            for c in 0 .. columns
            {
                let mut parsed = ParsedTemplate {
                    base: [0; 64],
                    overlay_from_n: [0; 64],
                    overlay_from_e: [0; 64],
                    overlay_from_s: [0; 64],
                    overlay_from_w: [0; 64],
                };

                let src_x0 = c * 8;
                let src_y0 = r * 8;

                let get_px = |x: u32, y: u32| -> [u8; 4] {
                    let idx = ((src_y0 + y) * width + (src_x0 + x)) as usize * 4;
                    [data[idx], data[idx + 1], data[idx + 2], data[idx + 3]]
                };

                // Check if the 8x8 block is completely empty (no opaque pixels).
                let mut is_empty = true;
                for y in 0 .. 8
                {
                    for x in 0 .. 8
                    {
                        if get_px(x, y)[3] > 0
                        {
                            is_empty = false;
                        }
                    }
                }

                if is_empty
                {
                    break;
                }

                // Base: 4x4 center of the 8x8 template (x=2..5, y=2..5)
                for y in 0 .. 4
                {
                    for x in 0 .. 4
                    {
                        let px = get_px(2 + x, 2 + y);
                        let out_idx = (y * 4 + x) as usize * 4;
                        parsed.base[out_idx .. out_idx + 4].copy_from_slice(&px);
                    }
                }

                // overlay_from_n: South edge of neighbor (y=6..7, x=2..5) drawn at Top (y=0..1)
                for y in 0 .. 2
                {
                    for x in 0 .. 4
                    {
                        let px = get_px(2 + x, 6 + y);
                        let out_idx = (y * 4 + x) as usize * 4;
                        parsed.overlay_from_n[out_idx .. out_idx + 4].copy_from_slice(&px);
                    }
                }

                // overlay_from_e: West edge of neighbor (x=0..1, y=2..5) drawn at Right (x=2..3)
                for y in 0 .. 4
                {
                    for x in 0 .. 2
                    {
                        let px = get_px(x, 2 + y);
                        let out_idx = (y * 4 + (x + 2)) as usize * 4;
                        parsed.overlay_from_e[out_idx .. out_idx + 4].copy_from_slice(&px);
                    }
                }

                // overlay_from_s: North edge of neighbor (y=0..1, x=2..5) drawn at Bottom (y=2..3)
                for y in 0 .. 2
                {
                    for x in 0 .. 4
                    {
                        let px = get_px(2 + x, y);
                        let out_idx = ((y + 2) * 4 + x) as usize * 4;
                        parsed.overlay_from_s[out_idx .. out_idx + 4].copy_from_slice(&px);
                    }
                }

                // overlay_from_w: East edge of neighbor (x=6..7, y=2..5) drawn at Left (x=0..1)
                for y in 0 .. 4
                {
                    for x in 0 .. 2
                    {
                        let px = get_px(6 + x, 2 + y);
                        let out_idx = (y * 4 + x) as usize * 4;
                        parsed.overlay_from_w[out_idx .. out_idx + 4].copy_from_slice(&px);
                    }
                }

                row_variations.push(parsed);
            }
            templates.push(row_variations);
        }

        return Self { templates, tile_size: 4 };
    }
}

// Opaque blit: copies one tile from the tileset into the chunk pixel buffer.
fn blit_tile(dst: &mut [u8], dst_w: u32, dst_x: u32, dst_y: u32, src_buf: &[u8; 64])
{
    let ts = 4;
    let stride = (ts as usize) * 4;

    for sy in 0 .. ts
    {
        let src_off = (sy * ts) as usize * 4;
        let dst_off = ((dst_y + sy) * dst_w + dst_x) as usize * 4;
        dst[dst_off .. dst_off + stride].copy_from_slice(&src_buf[src_off .. src_off + stride]);
    }
}

// Alpha-composited blit: draws a tile sprite on top of existing chunk pixels.
fn alpha_blit_tile(dst: &mut [u8], dst_w: u32, dst_x: u32, dst_y: u32, src_buf: &[u8; 64])
{
    let ts = 4;

    for sy in 0 .. ts
    {
        for sx in 0 .. ts
        {
            let si = (sy * ts + sx) as usize * 4;
            let di = ((dst_y + sy) * dst_w + (dst_x + sx)) as usize * 4;
            let sa = src_buf[si + 3] as u16;
            if sa == 0
            {
                continue;
            }
            if sa == 255
            {
                dst[di .. di + 4].copy_from_slice(&src_buf[si .. si + 4]);
            }
            else
            {
                let inv = 255 - sa;
                for c in 0 .. 3
                {
                    dst[di + c] =
                        ((dst[di + c] as u16 * inv + src_buf[si + c] as u16 * sa) / 255) as u8;
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
    let ts = tileset.tile_size;
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

            let row_vars = &tileset.templates[def.template_idx];
            let var_idx = (map_data.get_variation(gx, gy) * row_vars.len() as f32) as usize;
            let var_idx = var_idx.clamp(0, row_vars.len().saturating_sub(1));
            let template = &row_vars[var_idx];

            // Pixel coords: Y is flipped (tile y=0 -> bottom of image).
            let px = lx * ts;
            let py = (cs - 1 - ly) * ts;

            // Base tile (opaque copy).
            blit_tile(pixels, tex_w, px, py, &template.base);

            // Cardinal overlays (alpha-composited on top).
            for dir in 0 .. 4
            {
                if let Some(overlay_idx) =
                    autotile::compute_overlay_for_dir(gx, gy, dir, map_data, tile_registry)
                {
                    let (dx, dy) = autotile::CARDINAL_OFFSETS[dir];
                    let nx = (gx as i32 + dx) as u32;
                    let ny = (gy as i32 + dy) as u32;

                    let n_row_vars = &tileset.templates[overlay_idx];
                    let n_var_idx =
                        (map_data.get_variation(nx, ny) * n_row_vars.len() as f32) as usize;
                    let n_var_idx = n_var_idx.clamp(0, n_row_vars.len().saturating_sub(1));
                    let n_template = &n_row_vars[n_var_idx];

                    let overlay_buf = match dir
                    {
                        0 => &n_template.overlay_from_n,
                        1 => &n_template.overlay_from_e,
                        2 => &n_template.overlay_from_s,
                        3 => &n_template.overlay_from_w,
                        _ => unreachable!(),
                    };
                    alpha_blit_tile(pixels, tex_w, px, py, overlay_buf);
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
    let ts = tileset.tile_size;
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

    // Each template sprite is exactly 8x8 pixels.
    let template_size = 8;
    let columns = tileset_image.width() / template_size;
    let tileset = TilesetPixels::from_image(tileset_image, template_size, columns);

    let num_chunks = (map_data.chunks_x * map_data.chunks_y) as usize;
    let world_chunk = (map_data.chunk_size * map_data.tile_size) as f32;

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
                Sprite {
                    image: placeholder.clone(),
                    custom_size: Some(Vec2::splat(world_chunk)),
                    ..default()
                },
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
            let target_vis = if in_view && load_state.built[idx]
            {
                Visibility::Inherited
            }
            else
            {
                Visibility::Hidden
            };

            if *vis != target_vis
            {
                *vis = target_vis;
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
            .insert_resource(ClearColor(Color::srgb_u8(51, 112, 204)));
    }
}
