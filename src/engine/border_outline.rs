use bevy::prelude::*;

use crate::engine::consts::{CHUNK_SIZE, MAP_HEIGHT, MAP_WIDTH, TILE_SIZE};

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
// Semi-transparent white for the outline dots.
const OUTLINE_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, ALPHA as f32 / 255.0);
// Outline color as RGBA bytes for macro map blending.
pub const OUTLINE_COLOR_RGBA: [u8; 4] = [255, 255, 255, ALPHA];
// Z layer for the outline sprites (above overlays but below props).
const OUTLINE_Z: f32 = 0.5;

// Generates the ordered list of tile positions along the inner border, traversed anti-clockwise:
// bottom->right->top->left.
fn build_perimeter_path(inner_x0: i32, inner_y0: i32, inner_x1: i32, inner_y1: i32) -> Vec<IVec2>
{
    let mut path = Vec::new();

    // Bottom edge.
    for x in (inner_x0 ..= inner_x1).rev()
    {
        path.push(IVec2::new(x, inner_y0));
    }
    // Right edge.
    for y in ((inner_y0 + 1) ..= inner_y1).rev()
    {
        path.push(IVec2::new(inner_x1, y));
    }
    // Top edge.
    for x in inner_x0 ..= (inner_x1 - 1)
    {
        path.push(IVec2::new(x, inner_y1));
    }
    // Left edge.
    for y in (inner_y0 + 1) ..= (inner_y1 - 1)
    {
        path.push(IVec2::new(inner_x0, y));
    }

    return path;
}

// For a given frame offset (0..FRAME_COUNT), returns the set of tiles that should be lit.
fn compute_frame_tiles(path: &[IVec2], frame_offset: usize) -> Vec<IVec2>
{
    let mut tiles = Vec::new();

    for (i, pos) in path.iter().enumerate()
    {
        // Shift the pattern by frame_offset tiles anti-clockwise.
        let phase = (i + frame_offset) % FRAME_COUNT;
        if phase < SEGMENT_LEN as usize
        {
            tiles.push(*pos);
        }
    }

    return tiles;
}

// Converts a grid position to world position for a sprite, accounting for
// TilemapAnchor::Center (the tilemap is centered at origin).
fn grid_to_sprite_world(grid: IVec2) -> Vec3
{
    let map_w = (MAP_WIDTH * CHUNK_SIZE) as f32;
    let map_h = (MAP_HEIGHT * CHUNK_SIZE) as f32;
    let ts = TILE_SIZE as f32;

    return Vec3::new(
        (grid.x as f32 - map_w / 2.0) * ts + ts * 0.5,
        (grid.y as f32 - map_h / 2.0) * ts + ts * 0.5,
        OUTLINE_Z,
    );
}

// Root entity for one animation frame. Only one is visible at a time.
#[derive(Component)]
struct OutlineFrame
{
    index: usize,
}

// Resource tracking animation state.
#[derive(Resource)]
pub struct OutlineAnimation
{
    timer: Timer,
    pub current_frame: usize,
    pub frames: Vec<Vec<IVec2>>,
}

fn setup_outline(mut commands: Commands)
{
    let map_w = (MAP_WIDTH * CHUNK_SIZE) as i32;
    let map_h = (MAP_HEIGHT * CHUNK_SIZE) as i32;

    // Inner border: just inside the 1-tile non-modifiable ocean border.
    let inner_x0 = 1_i32;
    let inner_y0 = 1_i32;
    let inner_x1 = map_w - 2;
    let inner_y1 = map_h - 2;

    let path = build_perimeter_path(inner_x0, inner_y0, inner_x1, inner_y1);

    let ts = TILE_SIZE as f32;

    let mut all_frames = Vec::with_capacity(FRAME_COUNT);

    for frame_idx in 0 .. FRAME_COUNT
    {
        let tiles = compute_frame_tiles(&path, frame_idx);
        let visible = frame_idx == 0;

        // Spawn root entity for this frame.
        let frame_entity = commands
            .spawn((
                OutlineFrame { index: frame_idx },
                if visible { Visibility::Inherited } else { Visibility::Hidden },
                Transform::default(),
            ))
            .id();

        // Spawn child sprites for each lit tile in this frame.
        for pos in &tiles
        {
            let world_pos = grid_to_sprite_world(*pos);
            let child = commands
                .spawn((
                    Sprite {
                        color: OUTLINE_COLOR,
                        custom_size: Some(Vec2::new(ts, ts)),
                        ..default()
                    },
                    Transform::from_translation(world_pos),
                ))
                .id();
            commands.entity(frame_entity).add_child(child);
        }

        all_frames.push(tiles);
    }

    commands.insert_resource(OutlineAnimation {
        timer: Timer::from_seconds(OUTLINE_PERIOD, TimerMode::Repeating),
        current_frame: 0,
        frames: all_frames,
    });
}

fn animate_outline(
    time: Res<Time>,
    mut anim: ResMut<OutlineAnimation>,
    mut frames: Query<(&OutlineFrame, &mut Visibility)>,
)
{
    anim.timer.tick(time.delta());

    if anim.timer.just_finished()
    {
        let prev = anim.current_frame;
        anim.current_frame = (anim.current_frame + 1) % FRAME_COUNT;
        let next = anim.current_frame;

        for (frame, mut vis) in &mut frames
        {
            if frame.index == prev
            {
                *vis = Visibility::Hidden;
            }
            else if frame.index == next
            {
                *vis = Visibility::Inherited;
            }
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
