use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use super::StandardRenderLayer;
use crate::engine::{
    consts::{CHUNK_SIZE, MAP_HEIGHT, MAP_WIDTH, TILE_SIZE},
    coords::GridPos,
    prop::{AnimationState, PropRegistry, PropType},
    tile::{TileRegistry, TileType},
};

#[derive(Component)]
pub struct MacroMapDot
{
    pub color: [u8; 4],
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MapMode
{
    #[default]
    Standard,
    Macro,
}

#[derive(Resource)]
pub struct MacroMapData
{
    pub handle: Handle<Image>,
    pub tile_cache: Vec<u8>,
    pub width: i32,
    pub height: i32,
}

#[derive(Component)]
pub struct MacroMapSprite;

fn init_macro_engine(mut commands: Commands, mut images: ResMut<Assets<Image>>)
{
    let num_pixels = (MAP_WIDTH * MAP_HEIGHT * CHUNK_SIZE * CHUNK_SIZE) as usize;
    let tile_cache = vec![0, 0, 0, 255].repeat(num_pixels);

    let image = Image::new(
        Extent3d {
            width: MAP_WIDTH * CHUNK_SIZE,
            height: MAP_HEIGHT * CHUNK_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        tile_cache.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(image);

    commands.insert_resource(MacroMapData {
        handle: handle.clone(),
        tile_cache,
        width: (MAP_WIDTH * CHUNK_SIZE) as i32,
        height: (MAP_HEIGHT * CHUNK_SIZE) as i32,
    });

    commands.spawn((
        Sprite {
            image: handle,
            custom_size: Some(Vec2::new(
                (CHUNK_SIZE * MAP_WIDTH * TILE_SIZE) as f32,
                (CHUNK_SIZE * MAP_HEIGHT * TILE_SIZE) as f32,
            )),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::new(1.0, -1.0, 1.0)),
        Visibility::Hidden,
        MacroMapSprite,
    ));
}

fn update_tile_cache(
    mut map_data: ResMut<MacroMapData>,
    tile_registry: Res<TileRegistry>,
    tile_query: Query<(&GridPos, &TileType), Changed<TileType>>,
)
{
    for (pos, tile_type) in &tile_query
    {
        if pos.x >= 0 && pos.x < map_data.width && pos.y >= 0 && pos.y < map_data.height
        {
            let idx = ((pos.y * map_data.width) + pos.x) as usize * 4;
            let color = tile_registry.get_color(*tile_type);

            map_data.tile_cache[idx .. idx + 3].copy_from_slice(&color);
            map_data.tile_cache[idx + 3] = 255;
        }
    }
}

fn paint_macro_map(
    map_data: Res<MacroMapData>,
    prop_registry: Res<PropRegistry>,
    mut images: ResMut<Assets<Image>>,
    prop_query: Query<(&GridPos, &PropType, Option<&AnimationState>)>,
    dot_query: Query<(&GridPos, &MacroMapDot)>,
)
{
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

    data.copy_from_slice(&map_data.tile_cache);

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
}

fn handle_zoom_states(
    camera_query: Query<&Projection, (With<Camera2d>, Changed<Projection>)>,
    current_state: Res<State<MapMode>>,
    mut next_state: ResMut<NextState<MapMode>>,
)
{
    let threshold = 1.0;
    if let Ok(Projection::Orthographic(ortho)) = camera_query.single()
    {
        if *current_state.get() == MapMode::Standard && ortho.scale > threshold
        {
            next_state.set(MapMode::Macro);
        }
        else if *current_state.get() == MapMode::Macro && ortho.scale <= threshold
        {
            next_state.set(MapMode::Standard);
        }
    }
}

fn show_macro(mut q: Query<&mut Visibility, With<MacroMapSprite>>)
{
    for mut vis in &mut q
    {
        *vis = Visibility::Visible;
    }
}

fn hide_macro(mut q: Query<&mut Visibility, With<MacroMapSprite>>)
{
    for mut vis in &mut q
    {
        *vis = Visibility::Hidden;
    }
}

fn show_standard(mut q: Query<&mut Visibility, With<StandardRenderLayer>>)
{
    for mut vis in &mut q
    {
        *vis = Visibility::Inherited;
    }
}

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
            .add_systems(Update, update_tile_cache)
            .add_systems(Update, handle_zoom_states)
            .add_systems(OnEnter(MapMode::Macro), (show_macro, hide_standard))
            .add_systems(OnEnter(MapMode::Standard), (show_standard, hide_macro))
            .add_systems(Update, paint_macro_map.run_if(in_state(MapMode::Macro)));
    }
}
