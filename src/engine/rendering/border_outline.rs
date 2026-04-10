use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::tilemap::StandardRenderLayer;
use crate::engine::mapgen::MapData;

// Period of the outline animation in seconds.
const OUTLINE_PERIOD: f32 = 0.25;
// Length of each segment in tiles.
const SEGMENT_LEN: u32 = 4;
// Gap between segments in tiles.
const GAP_LEN: u32 = 1;
// Number of precalculated frames (length of segment + gap).
const FRAME_COUNT: usize = (SEGMENT_LEN + GAP_LEN) as usize;
// Alpha value for the outline color (as a byte value).
const ALPHA: u8 = 115;
// Outline color as RGBA bytes for macro map blending.
pub const OUTLINE_COLOR_RGBA: [u8; 4] = [255, 255, 255, ALPHA];
// Z layer for the outline sprites (above overlays but below props).
const OUTLINE_Z: f32 = 0.5;

// Builds the ordered list of tile positions along the inner perimeter of the map.
fn build_perimeter_path(inner_x0: i32, inner_y0: i32, inner_x1: i32, inner_y1: i32) -> Vec<IVec2>
{
    let mut path = Vec::new();

    for x in (inner_x0 ..= inner_x1).rev()
    {
        path.push(IVec2::new(x, inner_y0));
    }
    for y in ((inner_y0 + 1) ..= inner_y1).rev()
    {
        path.push(IVec2::new(inner_x1, y));
    }
    for x in inner_x0 ..= (inner_x1 - 1)
    {
        path.push(IVec2::new(x, inner_y1));
    }
    for y in (inner_y0 + 1) ..= (inner_y1 - 1)
    {
        path.push(IVec2::new(inner_x0, y));
    }

    return path;
}

// Selects which perimeter tiles are visible for a given animation frame offset.
fn compute_frame_tiles(path: &[IVec2], frame_offset: usize) -> Vec<IVec2>
{
    return path
        .iter()
        .enumerate()
        .filter(|(i, _)| (i + frame_offset) % FRAME_COUNT < SEGMENT_LEN as usize)
        .map(|(_, pos)| *pos)
        .collect();
}

// Holds pre-rendered outline frames and animation state.
#[derive(Resource)]
pub struct OutlineAnimation
{
    timer: Timer,
    // Index of the currently displayed animation frame.
    pub current_frame: usize,
    // Tile positions for each pre-computed frame.
    pub frames: Vec<Vec<IVec2>>,
    // Image handles for each pre-rendered frame texture.
    pub frame_handles: Vec<Handle<Image>>,
}

// Marker component for the outline sprite entity.
#[derive(Component)]
struct OutlineSprite;

// Pre-renders all outline animation frames and spawns the outline sprite.
fn setup_outline(mut commands: Commands, mut images: ResMut<Assets<Image>>, map_data: Res<MapData>)
{
    let map_w = map_data.width_tiles() as i32;
    let map_h = map_data.height_tiles() as i32;
    let ts = map_data.tile_size as f32;

    let inner_x0 = 1_i32;
    let inner_y0 = 1_i32;
    let inner_x1 = map_w - 2;
    let inner_y1 = map_h - 2;

    let path = build_perimeter_path(inner_x0, inner_y0, inner_x1, inner_y1);

    let w = map_w as u32;
    let h = map_h as u32;
    let mut all_frames = Vec::with_capacity(FRAME_COUNT);
    let mut frame_handles = Vec::with_capacity(FRAME_COUNT);

    for frame_idx in 0 .. FRAME_COUNT
    {
        let tiles = compute_frame_tiles(&path, frame_idx);

        // Build a transparent RGBA image with only the outline pixels set.
        let mut data = vec![0u8; (w * h * 4) as usize];
        for pos in &tiles
        {
            let idx = ((pos.y as u32 * w) + pos.x as u32) as usize * 4;
            data[idx] = 255;
            data[idx + 1] = 255;
            data[idx + 2] = 255;
            data[idx + 3] = ALPHA;
        }

        let image = Image::new(
            Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        let handle = images.add(image);
        frame_handles.push(handle);
        all_frames.push(tiles);
    }

    // Single sprite entity for the outline.
    commands.spawn((
        Sprite {
            image: frame_handles[0].clone(),
            custom_size: Some(Vec2::new(w as f32 * ts, h as f32 * ts)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, OUTLINE_Z).with_scale(Vec3::new(1.0, -1.0, 1.0)),
        OutlineSprite,
        StandardRenderLayer,
    ));

    commands.insert_resource(OutlineAnimation {
        timer: Timer::from_seconds(OUTLINE_PERIOD, TimerMode::Repeating),
        current_frame: 0,
        frames: all_frames,
        frame_handles,
    });
}

// Cycles through outline frames on a timer and swaps the sprite image.
fn animate_outline(
    time: Res<Time>,
    mut anim: ResMut<OutlineAnimation>,
    mut sprites: Query<&mut Sprite, With<OutlineSprite>>,
)
{
    anim.timer.tick(time.delta());

    if anim.timer.just_finished()
    {
        anim.current_frame = (anim.current_frame + 1) % FRAME_COUNT;
        let handle = anim.frame_handles[anim.current_frame].clone();
        for mut sprite in &mut sprites
        {
            sprite.image = handle.clone();
        }
    }
}

pub struct BorderOutlinePlugin;

impl Plugin for BorderOutlinePlugin
{
    fn build(&self, app: &mut App)
    {
        app.add_systems(PostStartup, setup_outline)
            .add_systems(Update, animate_outline);
    }
}
