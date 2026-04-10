use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::{
    border_outline::{OUTLINE_COLOR_RGBA, OutlineAnimation},
    tilemap::StandardRenderLayer,
};
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

// State machine controlling which rendering mode is active.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MapMode
{
    #[default]
    Standard,
    Macro,
}

// Holds the macro map image, a tile-color cache, and dimensions.
#[derive(Resource)]
pub struct MacroMapData
{
    // Handle to the GPU-side macro map image.
    pub handle: Handle<Image>,
    // CPU-side RGBA pixel cache containing only tile colors (no overlays).
    pub tile_cache: Vec<u8>,
    // Map dimensions in tiles.
    pub width: i32,
    pub height: i32,
    // Set to true when the tile cache changes and the image needs repainting.
    pub dirty: bool,
}

// Marker component for the macro map sprite entity.
#[derive(Component)]
pub struct MacroMapSprite;

// Initializes the macro map: builds the tile-color cache and spawns the macro map sprite.
fn init_macro_engine(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    map_data: Res<MapData>,
    tile_registry: Res<TileRegistry>,
)
{
    let w = map_data.width_tiles();
    let h = map_data.height_tiles();
    let num_pixels = (w * h) as usize;
    let mut tile_cache = vec![0u8; num_pixels * 4];

    for y in 0 .. h
    {
        for x in 0 .. w
        {
            let tile_type = map_data.get_tile(x, y);
            let def = tile_registry.tiles.get(&tile_type);
            let color = def.map(|d| d.macro_color).unwrap_or([0, 0, 0]);
            let idx = ((y * w) + x) as usize * 4;
            tile_cache[idx] = color[0];
            tile_cache[idx + 1] = color[1];
            tile_cache[idx + 2] = color[2];
            tile_cache[idx + 3] = 255;
        }
    }

    let image = Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        tile_cache.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(image);

    let ts = map_data.tile_size as f32;

    commands.insert_resource(MacroMapData {
        handle: handle.clone(),
        tile_cache,
        width: w as i32,
        height: h as i32,
        dirty: true,
    });

    commands.spawn((
        Sprite {
            image: handle,
            custom_size: Some(Vec2::new(w as f32 * ts, h as f32 * ts)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::new(1.0, -1.0, 1.0)),
        Visibility::Hidden,
        MacroMapSprite,
    ));
}

// Updates the tile-color cache for chunks whose tiles have been painted.
fn update_tile_cache(
    mut map_data: ResMut<MapData>,
    mut macro_data: ResMut<MacroMapData>,
    tile_registry: Res<TileRegistry>,
)
{
    let cs = map_data.chunk_size;

    for cy in 0 .. map_data.chunks_y
    {
        for cx in 0 .. map_data.chunks_x
        {
            if !map_data.take_macro_chunk_dirty(cx, cy)
            {
                continue;
            }

            let x0 = cx * cs;
            let y0 = cy * cs;
            for ly in 0 .. cs
            {
                for lx in 0 .. cs
                {
                    let gx = x0 + lx;
                    let gy = y0 + ly;
                    let tile_type = map_data.get_tile(gx, gy);
                    let color = tile_registry
                        .tiles
                        .get(&tile_type)
                        .map(|d| d.macro_color)
                        .unwrap_or([0, 0, 0]);
                    let pixel = (gy as usize * macro_data.width as usize + gx as usize) * 4;
                    macro_data.tile_cache[pixel] = color[0];
                    macro_data.tile_cache[pixel + 1] = color[1];
                    macro_data.tile_cache[pixel + 2] = color[2];
                    macro_data.tile_cache[pixel + 3] = 255;
                }
            }
            macro_data.dirty = true;
        }
    }
}

// Composites the macro map image from the tile cache, prop colors, dots, and outline.
fn paint_macro_map(
    mut map_data: ResMut<MacroMapData>,
    prop_registry: Res<PropRegistry>,
    outline_anim: Res<OutlineAnimation>,
    mut images: ResMut<Assets<Image>>,
    prop_query: Query<(&GridPos, &PropType, Option<&AnimationState>)>,
    dot_query: Query<(&GridPos, &MacroMapDot)>,
)
{
    if !map_data.dirty && !outline_anim.is_changed()
    {
        return;
    }
    map_data.dirty = false;
    let Some(image) = images.get_mut(&map_data.handle)
    else
    {
        return;
    };
    let Some(data) = image.data.as_mut()
    else
    {
        return;
    };

    // Start from the clean tile cache.
    data.copy_from_slice(&map_data.tile_cache);

    // Overlay prop macro colors.
    for (pos, prop_type, anim) in &prop_query
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
                    if px >= 0 && px < map_data.width && py >= 0 && py < map_data.height
                    {
                        let idx = ((py * map_data.width) + px) as usize * 4;
                        if data[idx .. idx + 4] == map_data.tile_cache[idx .. idx + 4]
                        {
                            data[idx .. idx + 4].copy_from_slice(&colors[i]);
                        }
                    }
                    i += 1;
                }
            }
        }
    }

    // Overlay single-pixel dots (entities like humans, animals).
    for (pos, dot) in &dot_query
    {
        if pos.x >= 0 && pos.x < map_data.width && pos.y >= 0 && pos.y < map_data.height
        {
            let idx = ((pos.y * map_data.width) + pos.x) as usize * 4;
            if data[idx .. idx + 4] == map_data.tile_cache[idx .. idx + 4]
            {
                data[idx .. idx + 4].copy_from_slice(&dot.color);
            }
        }
    }

    // Paint the animated border outline on top.
    let outline_rgba = OUTLINE_COLOR_RGBA;
    let alpha = outline_rgba[3] as u16;
    let inv_alpha = 255 - alpha;
    if let Some(frame_tiles) = outline_anim.frames.get(outline_anim.current_frame)
    {
        for pos in frame_tiles
        {
            if pos.x >= 0 && pos.x < map_data.width && pos.y >= 0 && pos.y < map_data.height
            {
                let idx = ((pos.y * map_data.width) + pos.x) as usize * 4;
                for c in 0 .. 3
                {
                    data[idx + c] = ((data[idx + c] as u16 * inv_alpha
                        + outline_rgba[c] as u16 * alpha)
                        / 255) as u8;
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

// Shows the macro map sprite and forces a repaint.
fn show_macro(
    mut q: Query<&mut Visibility, With<MacroMapSprite>>,
    mut macro_data: ResMut<MacroMapData>,
)
{
    for mut vis in &mut q
    {
        *vis = Visibility::Visible;
    }
    // Force a repaint on the first macro frame.
    macro_data.dirty = true;
}

// Hides the macro map sprite.
fn hide_macro(mut q: Query<&mut Visibility, With<MacroMapSprite>>)
{
    for mut vis in &mut q
    {
        *vis = Visibility::Hidden;
    }
}

// Makes all standard-mode entities visible.
fn show_standard(mut q: Query<&mut Visibility, With<StandardRenderLayer>>)
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
            .add_systems(Update, handle_zoom_states)
            .add_systems(OnEnter(MapMode::Macro), (show_macro, hide_standard))
            .add_systems(OnEnter(MapMode::Standard), (show_standard, hide_macro))
            .add_systems(Update, paint_macro_map.run_if(in_state(MapMode::Macro)));
    }
}
