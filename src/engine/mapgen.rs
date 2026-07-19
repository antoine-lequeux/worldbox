use std::{
    cmp::Ordering,
    collections::{BinaryHeap, VecDeque},
};

use bevy::prelude::*;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

use crate::engine::{
    consts::{CHUNK_SIZE, LANDMASS_DENSITY, MAP_HEIGHT, MAP_SEED, MAP_WIDTH, PROP_Z, TILE_SIZE},
    tile::TileType,
};

// Central resource holding the full tile map, dimensions, and dirty flags.
#[derive(Resource)]
pub struct MapData
{
    // Size of one tile in pixels.
    pub tile_size: u32,
    // Number of tiles per chunk side.
    pub chunk_size: u32,
    // Flat array of tile types, row-major (y * width + x).
    tiles: Vec<TileType>,
    // Map dimensions in tiles.
    width: u32,
    height: u32,
    // Map dimensions in chunks.
    pub chunks_x: u32,
    pub chunks_y: u32,
    // Per-chunk dirty flag: set when any tile inside changes.
    pub dirty_chunks: Vec<bool>,
    // Separate dirty flags for the macro map (not cleared by autotile rebuild).
    pub macro_dirty_chunks: Vec<bool>,
    // Per-tile random variation value (0.0 to 1.0).
    pub variations: Vec<f32>,
}

impl MapData
{
    // Returns the map width in tiles.
    pub fn width_tiles(&self) -> u32
    {
        return self.width;
    }

    // Returns the map height in tiles.
    pub fn height_tiles(&self) -> u32
    {
        return self.height;
    }

    // Returns the tile type at the given tile coordinates.
    pub fn get_tile(&self, x: u32, y: u32) -> TileType
    {
        return self.tiles[(y * self.width + x) as usize];
    }

    // Returns the random variation float [0.0..1.0) for the given tile coordinates.
    pub fn get_variation(&self, x: u32, y: u32) -> f32
    {
        return self.variations[(y * self.width + x) as usize];
    }

    // Sets a tile and marks the containing chunk dirty. Returns true if the type changed.
    // Also dirties neighbour chunks at boundaries (overlay edges depend on neighbours).
    pub fn set_tile(&mut self, x: u32, y: u32, tile_type: TileType) -> bool
    {
        let idx = (y * self.width + x) as usize;
        if self.tiles[idx] == tile_type
        {
            return false;
        }
        self.tiles[idx] = tile_type;
        let cx = x / self.chunk_size;
        let cy = y / self.chunk_size;
        let ci = (cy * self.chunks_x + cx) as usize;
        self.dirty_chunks[ci] = true;
        self.macro_dirty_chunks[ci] = true;

        // Dirty neighbour chunks when the tile sits on a chunk boundary.
        let lx = x % self.chunk_size;
        let ly = y % self.chunk_size;
        if lx == 0 && cx > 0
        {
            self.dirty_chunks[(cy * self.chunks_x + (cx - 1)) as usize] = true;
        }
        if lx == self.chunk_size - 1 && cx + 1 < self.chunks_x
        {
            self.dirty_chunks[(cy * self.chunks_x + (cx + 1)) as usize] = true;
        }
        if ly == 0 && cy > 0
        {
            self.dirty_chunks[((cy - 1) * self.chunks_x + cx) as usize] = true;
        }
        if ly == self.chunk_size - 1 && cy + 1 < self.chunks_y
        {
            self.dirty_chunks[((cy + 1) * self.chunks_x + cx) as usize] = true;
        }
        return true;
    }

    // Converts a world-space position to tile-grid coordinates.
    pub fn world_to_grid(&self, world_pos: Vec2) -> IVec2
    {
        let half_w = (self.width as f32 * self.tile_size as f32) / 2.0;
        let half_h = (self.height as f32 * self.tile_size as f32) / 2.0;
        let x = ((world_pos.x + half_w) / self.tile_size as f32).floor() as i32;
        let y = ((world_pos.y + half_h) / self.tile_size as f32).floor() as i32;
        return IVec2::new(x, y);
    }

    // Converts a grid position to world-space center for a prop of the given size.
    pub fn grid_to_prop_world(&self, grid: IVec2, size_tiles: UVec2) -> Vec3
    {
        let half_w = (self.width as f32 * self.tile_size as f32) / 2.0;
        let half_h = (self.height as f32 * self.tile_size as f32) / 2.0;
        let ts = self.tile_size as f32;
        let x = (grid.x as f32 * ts) - half_w + (size_tiles.x as f32 * ts / 2.0);
        let y = (grid.y as f32 * ts) - half_h + (size_tiles.y as f32 * ts / 2.0);
        return Vec3::new(x, y, PROP_Z);
    }

    // Reads and clears the dirty flag for the given chunk. Returns true if it was dirty.
    pub fn take_chunk_dirty(&mut self, cx: u32, cy: u32) -> bool
    {
        let idx = (cy * self.chunks_x + cx) as usize;
        let was = self.dirty_chunks[idx];
        self.dirty_chunks[idx] = false;
        return was;
    }

    // Reads and clears the macro dirty flag for the given chunk.
    pub fn take_macro_chunk_dirty(&mut self, cx: u32, cy: u32) -> bool
    {
        let idx = (cy * self.chunks_x + cx) as usize;
        let was = self.macro_dirty_chunks[idx];
        self.macro_dirty_chunks[idx] = false;
        return was;
    }

    // Returns the world-space bottom-left corner of a chunk.
    pub fn chunk_world_origin(&self, cx: u32, cy: u32) -> Vec2
    {
        let half_w = (self.width as f32 * self.tile_size as f32) / 2.0;
        let half_h = (self.height as f32 * self.tile_size as f32) / 2.0;
        let ts = self.tile_size as f32;
        return Vec2::new(
            cx as f32 * self.chunk_size as f32 * ts - half_w,
            cy as f32 * self.chunk_size as f32 * ts - half_h,
        );
    }
}

// Min-heap node (wraps f32 cost + flat pixel index).
#[derive(Clone, PartialEq)]
struct He(f32, usize);
impl Eq for He {}
impl PartialOrd for He
{
    fn partial_cmp(&self, o: &Self) -> Option<Ordering>
    {
        return Some(self.cmp(o));
    }
}
impl Ord for He
{
    // Reverse so BinaryHeap becomes a min-heap.
    fn cmp(&self, o: &Self) -> Ordering
    {
        return o.0.partial_cmp(&self.0).unwrap_or(Ordering::Equal);
    }
}

// Catmull-Rom cubic interpolation.
#[inline(always)]
fn cubic(a: f32, b: f32, c: f32, d: f32, t: f32) -> f32
{
    return b + 0.5
        * t
        * (c - a + t * (2.0 * a - 5.0 * b + 4.0 * c - d + t * (3.0 * (b - c) + d - a)));
}

// Separable 1D bicubic upsampling.
fn upscale_bicubic(grid: &[f32], gw: usize, gh: usize, w: usize, h: usize) -> Vec<f32>
{
    let mut temp = vec![0f32; w * gh];
    let gw_f = gw as f32;
    let w_f = w as f32;

    let mut x_coords = Vec::with_capacity(w);
    for x in 0 .. w
    {
        let gx = x as f32 * gw_f / w_f;
        let ix = gx.floor() as i32;
        let tx = gx - ix as f32;
        x_coords.push((ix, tx));
    }

    temp.par_chunks_mut(w).enumerate().for_each(|(y, out_row)| {
        let in_row = &grid[y * gw .. (y + 1) * gw];
        let cx = |v: i32| v.clamp(0, gw as i32 - 1) as usize;
        for x in 0 .. w
        {
            let (ix, tx) = x_coords[x];
            out_row[x] = cubic(
                in_row[cx(ix - 1)],
                in_row[cx(ix)],
                in_row[cx(ix + 1)],
                in_row[cx(ix + 2)],
                tx,
            );
        }
    });

    let mut out = vec![0f32; w * h];
    let gh_f = gh as f32;
    let h_f = h as f32;

    out.par_chunks_mut(w).enumerate().for_each(|(y, out_row)| {
        let gy = y as f32 * gh_f / h_f;
        let iy = gy.floor() as i32;
        let ty = gy - iy as f32;
        let cy = |v: i32| v.clamp(0, gh as i32 - 1) as usize;

        let r0 = cy(iy - 1) * w;
        let r1 = cy(iy) * w;
        let r2 = cy(iy + 1) * w;
        let r3 = cy(iy + 2) * w;

        for x in 0 .. w
        {
            out_row[x] = cubic(temp[r0 + x], temp[r1 + x], temp[r2 + x], temp[r3 + x], ty);
        }
    });
    return out;
}

// Fractal octave noise with deterministic RNG and parallel bicubic upsampling.
fn fractal_noise(
    w: usize,
    h: usize,
    base_cell: usize,
    octaves: usize,
    pers: f32,
    rng: &mut ChaCha8Rng,
) -> Vec<f32>
{
    let mut out = vec![0f32; w * h];
    let mut amp = 1f32;
    let mut max_amp = 0f32;

    for oct in 0 .. octaves
    {
        let cell = (base_cell >> oct).max(1);
        let gw = (w / cell).max(1);
        let gh = (h / cell).max(1);

        let mut grid = vec![0f32; gw * gh];
        grid.iter_mut().for_each(|v| *v = rng.random::<f32>());

        let upscaled = upscale_bicubic(&grid, gw, gh, w, h);

        out.par_iter_mut().zip(upscaled).for_each(|(v, u)| {
            *v += u * amp;
        });

        max_amp += amp;
        amp *= pers;
    }

    let inv = 1.0 / max_amp;
    out.par_iter_mut().for_each(|v| *v *= inv);
    return out;
}

// Fast approximation of Gaussian Blur using 3 passes of Separable Box Blur.
fn gaussian_blur(src: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32>
{
    let w_ideal = (12.0 * sigma * sigma / 3.0 + 1.0).sqrt();
    let mut r = (w_ideal.floor() as usize) / 2;
    if r < 1
    {
        r = 1;
    }

    let mut current = src.to_vec();
    let mut transposed = vec![0f32; w * h];

    let blur_pass = |input: &[f32], output: &mut [f32], width: usize, _height: usize| {
        let iarr = 1.0 / (2 * r + 1) as f32;
        output
            .par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                let src_row = &input[y * width .. (y + 1) * width];
                let mut sum = 0.0;
                let first = src_row[0];
                let last = src_row[width - 1];

                for i in 0 ..= r
                {
                    sum += src_row[i];
                }
                for _ in 1 ..= r
                {
                    sum += first;
                }

                for x in 0 .. width
                {
                    row[x] = sum * iarr;
                    let left = if x >= r { src_row[x - r] } else { first };
                    let right = if x + r + 1 < width { src_row[x + r + 1] } else { last };
                    sum += right - left;
                }
            });
    };

    let transpose = |input: &[f32], output: &mut [f32], src_w: usize, src_h: usize| {
        output
            .par_chunks_mut(src_h)
            .enumerate()
            .for_each(|(x, out_row)| {
                for y in 0 .. src_h
                {
                    out_row[y] = input[y * src_w + x];
                }
            });
    };

    for _ in 0 .. 3
    {
        let mut temp_h = vec![0f32; w * h];
        blur_pass(&current, &mut temp_h, w, h);

        transpose(&temp_h, &mut transposed, w, h);

        let mut temp_v = vec![0f32; w * h];
        blur_pass(&transposed, &mut temp_v, h, w);

        transpose(&temp_v, &mut current, h, w);
    }

    return current;
}

// 2-pass Chamfer distance transform (Chebyshev distance).
fn bfs_dt(mask: &[bool], w: usize, h: usize) -> Vec<f32>
{
    let n = w * h;
    let mut dist = vec![f32::MAX; n];

    for i in 0 .. n
    {
        if mask[i]
        {
            dist[i] = 0.0;
        }
    }

    // Pass 1: top-left to bottom-right
    for y in 0 .. h
    {
        for x in 0 .. w
        {
            let i = y * w + x;
            let mut d = dist[i];

            if y > 0
            {
                if x > 0
                {
                    d = d.min(dist[(y - 1) * w + (x - 1)] + 1.0);
                }
                d = d.min(dist[(y - 1) * w + x] + 1.0);
                if x + 1 < w
                {
                    d = d.min(dist[(y - 1) * w + (x + 1)] + 1.0);
                }
            }
            if x > 0
            {
                d = d.min(dist[y * w + (x - 1)] + 1.0);
            }

            dist[i] = d;
        }
    }

    // Pass 2: bottom-right to top-left
    for y in (0 .. h).rev()
    {
        for x in (0 .. w).rev()
        {
            let i = y * w + x;
            let mut d = dist[i];

            if y + 1 < h
            {
                if x > 0
                {
                    d = d.min(dist[(y + 1) * w + (x - 1)] + 1.0);
                }
                d = d.min(dist[(y + 1) * w + x] + 1.0);
                if x + 1 < w
                {
                    d = d.min(dist[(y + 1) * w + (x + 1)] + 1.0);
                }
            }
            if x + 1 < w
            {
                d = d.min(dist[y * w + (x + 1)] + 1.0);
            }

            dist[i] = d;
        }
    }

    return dist;
}

// 4-connected component labelling.
fn label_comp(mask: &[bool], w: usize, h: usize) -> (Vec<u32>, Vec<u32>)
{
    let n = w * h;
    let mut labels = vec![0u32; n];
    let mut sizes = vec![0u32; 1];
    let mut lbl = 0u32;
    let mut q: VecDeque<usize> = VecDeque::with_capacity(1 << 16);

    for start in 0 .. n
    {
        if !mask[start] || labels[start] != 0
        {
            continue;
        }
        lbl += 1;
        labels[start] = lbl;
        q.push_back(start);
        let mut sz = 0u32;

        while let Some(ci) = q.pop_front()
        {
            sz += 1;
            let (cy, cx) = (ci / w, ci % w);
            for dy in -1i32 ..= 1
            {
                for dx in -1i32 ..= 1
                {
                    if (dy == 0 && dx == 0) || (dy != 0 && dx != 0)
                    {
                        continue;
                    }
                    let ny = cy as i32 + dy;
                    let nx = cx as i32 + dx;
                    if ny < 0 || ny >= h as i32 || nx < 0 || nx >= w as i32
                    {
                        continue;
                    }
                    let ni = ny as usize * w + nx as usize;
                    if mask[ni] && labels[ni] == 0
                    {
                        labels[ni] = lbl;
                        q.push_back(ni);
                    }
                }
            }
        }
        sizes.push(sz);
    }
    return (labels, sizes);
}

struct AStarData
{
    cost: Vec<f32>,
    prev: Vec<usize>,
    touched: Vec<usize>,
}

impl AStarData
{
    fn new(n: usize) -> Self
    {
        return Self {
            cost: vec![f32::MAX; n],
            prev: vec![0; n],
            touched: Vec::with_capacity(8192),
        };
    }

    fn set_cost(&mut self, i: usize, c: f32, p: usize)
    {
        if self.cost[i] == f32::MAX
        {
            self.touched.push(i);
        }
        self.cost[i] = c;
        self.prev[i] = p;
    }

    fn get_cost(&self, i: usize) -> f32
    {
        return self.cost[i];
    }

    fn clear(&mut self)
    {
        for &i in &self.touched
        {
            self.cost[i] = f32::MAX;
        }
        self.touched.clear();
    }
}

// A* river carving with sparse cost map. Returns Some(target_lake_label) on success.
fn carve_river(
    w: usize,
    h: usize,
    elev: &[f32],
    wmask: &[bool],
    is_riv: &mut [bool],
    flow: &mut [f32],
    lbl_w: &[u32],
    wsize: &[u32],
    dtw: &[f32],
    lake_in: &mut Vec<f32>,
    ocean_lbl: u32,
    rng: &mut ChaCha8Rng,
    astar: &mut AStarData,
    sy: usize,
    sx: usize,
    rflow: f32,
    src_lbl: Option<u32>,
) -> Option<u32>
{
    let si = sy * w + sx;
    let mut heap: BinaryHeap<He> = BinaryHeap::new();

    astar.set_cost(si, 0.0, si);
    heap.push(He(dtw[si] * 0.4, si));
    let mut tgt = usize::MAX;

    'search: while let Some(He(p, ci)) = heap.pop()
    {
        if p > astar.get_cost(ci) + dtw[ci] * 0.4 + 1e-5
        {
            continue;
        }

        if wmask[ci]
        {
            if let Some(sl) = src_lbl
            {
                if lbl_w[ci] == sl
                {
                    continue;
                }
            }
            // Ignore tiny puddles (like noise on the beach) so rivers reach real bodies of water
            if lbl_w[ci] == ocean_lbl
                || lbl_w[ci] == 0
                || (lbl_w[ci] as usize) < wsize.len() && wsize[lbl_w[ci] as usize] > 10
            {
                tgt = ci;
                break 'search;
            }
        }

        let (cy, cx) = (ci / w, ci % w);
        for dy in -1i32 ..= 1
        {
            for dx in -1i32 ..= 1
            {
                if dy == 0 && dx == 0
                {
                    continue;
                }
                let ny = cy as i32 + dy;
                let nx = cx as i32 + dx;
                if ny < 0 || ny >= h as i32 || nx < 0 || nx >= w as i32
                {
                    continue;
                }
                let ni = ny as usize * w + nx as usize;

                if let Some(sl) = src_lbl
                {
                    if wmask[ni] && lbl_w[ni] == sl
                    {
                        continue;
                    }
                }

                let zdiff = elev[ni] - elev[ci];
                let step = if is_riv[ni]
                {
                    0.1f32
                }
                else
                {
                    let base = if zdiff > 0.0 { 10.0 + zdiff * 500.0 } else { 1.0 + zdiff * 10.0 };
                    (base + rng.random_range(0.5f32 .. 3.0)).max(0.1)
                };

                let nc = astar.get_cost(ci) + step;
                if nc < astar.get_cost(ni)
                {
                    astar.set_cost(ni, nc, ci);
                    heap.push(He(nc + dtw[ni] * 0.4, ni));
                }
            }
        }
    }

    if tgt == usize::MAX
    {
        astar.clear();
        return None;
    }

    let mut path = vec![tgt];
    let mut cur = tgt;
    loop
    {
        let p = astar.prev[cur];
        if p == cur
        {
            break;
        }
        cur = p;
        path.push(cur);
        if cur == si
        {
            break;
        }
    }
    path.reverse();

    for i in 0 .. path.len() - 1
    {
        let pi = path[i];
        is_riv[pi] = true;
        flow[pi] += rflow;

        let ni = path[i + 1];
        let (ry, rx) = (pi / w, pi % w);
        let (ny, nx) = (ni / w, ni % w);
        if (ry as i32 - ny as i32).abs() == 1 && (rx as i32 - nx as i32).abs() == 1
        {
            let corner1 = ry * w + nx;
            is_riv[corner1] = true;
            flow[corner1] += rflow;

            let corner2 = ny * w + rx;
            is_riv[corner2] = true;
            flow[corner2] += rflow;
        }
    }

    let tlbl = lbl_w[tgt];
    astar.clear();

    if tlbl != ocean_lbl && tlbl != 0
    {
        if lake_in.len() <= tlbl as usize
        {
            lake_in.resize(tlbl as usize + 1, 0.0);
        }
        lake_in[tlbl as usize] += rflow;
        return Some(tlbl);
    }
    return None;
}

fn percentile(data: &[f32], p: f32) -> f32
{
    let mut v = data.to_vec();
    v.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    return v[((p / 100.0 * (v.len() as f32 - 1.0)) as usize).min(v.len() - 1)];
}

fn shuffle<T>(v: &mut [T], rng: &mut ChaCha8Rng)
{
    for i in (1 .. v.len()).rev()
    {
        v.swap(i, rng.random_range(0 ..= i));
    }
}

// Runs the full terrain generator and builds a MapData resource.
fn generate_map() -> MapData
{
    let width = MAP_WIDTH * CHUNK_SIZE;
    let height = MAP_HEIGHT * CHUNK_SIZE;
    let mw = width as usize;
    let mh = height as usize;
    let nc = LANDMASS_DENSITY as usize;
    let n = mw * mh;
    let mut rng = ChaCha8Rng::seed_from_u64(MAP_SEED);

    info!(
        "Generating world map ({}x{} tiles, {}x{} chunks, seed={})...",
        width, height, MAP_WIDTH, MAP_HEIGHT, MAP_SEED
    );

    // Continent centers.
    info!("Starting landmasses from {nc} centers...");
    let mg = 250usize;
    let mut centers: Vec<(f64, f64)> = Vec::new();
    let map_area = (mw * mh) as f64;
    let avg_area_per_center = map_area / nc as f64;
    // Dynamic minimum distance to fit all centers, but still allow some clustering.
    let mut md = (avg_area_per_center.sqrt() * 0.4).clamp(100.0, 600.0);
    let mut att = 0usize;

    while centers.len() < nc
    {
        let px = rng.random_range(mg as f64 .. (mw - mg) as f64);
        let py = rng.random_range(mg as f64 .. (mh - mg) as f64);
        if centers
            .iter()
            .all(|&(cy, cx)| (px - cx).powi(2) + (py - cy).powi(2) > md * md)
        {
            centers.push((py, px));
            att = 0;
        }
        else
        {
            att += 1;
            if att > 50
            {
                md = (md - 10.0).max(10.0);
                att = 0;
            }
        }
    }

    // Calculate sprawl vectors to push continents toward empty spaces.
    let mut sprawls: Vec<(f64, f64)> = vec![(0.0, 0.0); nc];
    for i in 0 .. nc
    {
        let (cy, cx) = centers[i];
        let mut vx = 0.0;
        let mut vy = 0.0;
        for j in 0 .. nc
        {
            if i == j
            {
                continue;
            }
            let (ojy, ojx) = centers[j];
            let dy = cy - ojy;
            let dx = cx - ojx;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let force = 200000.0 / dist; // Repel inversely proportional to distance
            vx += (dx / dist) * force;
            vy += (dy / dist) * force;
        }
        // Also repel from borders to push them inward if they are on the edge.
        let border_force = 100000.0;
        vx += border_force / cx.max(1.0) - border_force / (mw as f64 - cx).max(1.0);
        vy += border_force / cy.max(1.0) - border_force / (mh as f64 - cy).max(1.0);

        let v_len = (vx * vx + vy * vy).sqrt().max(0.001);
        // Normalize and scale the sprawl vector by a dynamic amount.
        let sprawl_mag = rng.random_range(0.4 .. 1.0) * md;
        sprawls[i] = (vy / v_len * sprawl_mag, vx / v_len * sprawl_mag);
    }

    // Generate Voronoi sub-plates for continents.
    info!("Generating continent sub-plates...");
    let mut cont: Vec<(i64, i64)> = Vec::new();

    for i in 0 .. nc
    {
        let (cy, cx) = centers[i];
        let (spy, spx) = sprawls[i];
        // The scale/complexity of the landmass
        let num_sub_plates = rng.random_range(5 ..= 20);
        let base_spread = rng.random_range(0.3 .. 0.6) * md;

        // Push the core of the continent
        cont.push((cy as i64, cx as i64));

        for _ in 1 .. num_sub_plates
        {
            let offset_y = spy * rng.random_range(-0.2 .. 1.2);
            let offset_x = spx * rng.random_range(-0.2 .. 1.2);
            let rn_y = rng.random_range(-1.0 .. 1.0) + rng.random_range(-1.0 .. 1.0);
            let rn_x = rng.random_range(-1.0 .. 1.0) + rng.random_range(-1.0 .. 1.0);
            let py = (cy + offset_y + rn_y * base_spread) as i64;
            let px = (cx + offset_x + rn_x * base_spread) as i64;
            cont.push((py.clamp(0, mh as i64 - 1), px.clamp(0, mw as i64 - 1)));
        }
    }

    let num_c = cont.len() as u32;

    // Fill the rest with ocean plates.
    info!("Filling remaining space with ocean plates...");
    let mut ocean: Vec<(i64, i64)> = Vec::new();
    let num_ocean_plates = 350; // High density to carve out oceans.
    for _ in 0 .. num_ocean_plates
    {
        ocean.push((rng.random_range(0 .. mh) as i64, rng.random_range(0 .. mw) as i64));
    }

    let plates: Vec<(f32, f32)> = cont
        .iter()
        .chain(ocean.iter())
        .map(|&(y, x)| (y as f32, x as f32))
        .collect();

    // Domain-warped Voronoi.
    info!("Computing tectonic fault lines...");
    let wy: Vec<f32> = fractal_noise(mw, mh, 150, 4, 0.5, &mut rng)
        .iter()
        .map(|&v| (v - 0.5) * 150.0)
        .collect();
    let wx: Vec<f32> = fractal_noise(mw, mh, 150, 4, 0.5, &mut rng)
        .iter()
        .map(|&v| (v - 0.5) * 150.0)
        .collect();

    // Optimize by dividing into 64x64 chunks to filter candidate plates.
    let chunk_size = 64usize;
    let cx_count = (mw + chunk_size - 1) / chunk_size;
    let cy_count = (mh + chunk_size - 1) / chunk_size;

    // Max displacement is 75.0 (since noise is -0.5 to 0.5, times 150.0).
    let max_warp = 75.0_f32;
    // Max physical distance from center of a 64x64 chunk to any point in the chunk.
    let chunk_radius = (chunk_size as f32 * 0.5) * 1.4142135;
    let effective_radius = chunk_radius + max_warp;

    // For each chunk, precompute candidate plates.
    let chunk_candidates: Vec<Vec<usize>> = (0 .. cy_count * cx_count)
        .into_par_iter()
        .map(|ci| {
            let cy = ci / cx_count;
            let cx = ci % cx_count;
            let center_y = (cy * chunk_size + chunk_size / 2) as f32;
            let center_x = (cx * chunk_size + chunk_size / 2) as f32;

            // Find 2nd closest plate to center.
            let (mut d1, mut d2) = (f32::MAX, f32::MAX);
            for (pi, &(py, px)) in plates.iter().enumerate()
            {
                let wt = if pi < num_c as usize { 0.3 } else { 1.0 };
                let d = ((center_y - py).powi(2) + (center_x - px).powi(2)) * wt;
                if d < d1
                {
                    d2 = d1;
                    d1 = d;
                }
                else if d < d2
                {
                    d2 = d;
                }
            }

            // Any point in chunk can be at most effective_radius away from center.
            // Filter plates based on whether they could possibly beat d2.
            let mut candidates = Vec::new();
            for (pi, &(py, px)) in plates.iter().enumerate()
            {
                let wt = if pi < num_c as usize { 0.3 } else { 1.0 };
                let dist_to_center = ((center_y - py).powi(2) + (center_x - px).powi(2)).sqrt();
                let min_possible_dist = (dist_to_center - effective_radius).max(0.0);
                let min_possible_d2 = min_possible_dist * min_possible_dist * wt;

                if min_possible_d2 <= d2
                {
                    candidates.push(pi);
                }
            }
            candidates
        })
        .collect();

    let voro: Vec<(u32, u32, f32, f32)> = (0 .. n)
        .into_par_iter()
        .map(|i| {
            let (y, x) = (i / mw, i % mw);
            let (yw, xw) = (y as f32 + wy[i], x as f32 + wx[i]);
            let (mut d1, mut d2, mut p1, mut p2) = (f32::MAX, f32::MAX, 0u32, 0u32);

            let cy = y / chunk_size;
            let cx = x / chunk_size;
            let ci = cy * cx_count + cx;
            let candidates = &chunk_candidates[ci];

            for &pi in candidates
            {
                let (py, px) = plates[pi];
                let wt = if pi < num_c as usize { 0.3 } else { 1.0 };
                let d = ((yw - py).powi(2) + (xw - px).powi(2)) * wt;
                if d < d1
                {
                    d2 = d1;
                    p2 = p1;
                    d1 = d;
                    p1 = pi as u32;
                }
                else if d < d2
                {
                    d2 = d;
                    p2 = pi as u32;
                }
            }
            (p1, p2, d1.sqrt(), d2.sqrt())
        })
        .collect();

    let closest: Vec<u32> = voro.iter().map(|r| r.0).collect();
    let second: Vec<u32> = voro.iter().map(|r| r.1).collect();
    let bnd: Vec<f32> = voro
        .iter()
        .map(|r| (1.0 - (r.3 - r.2) / 60.0).clamp(0.0, 1.0))
        .collect();

    let is_cl: Vec<bool> = closest.iter().map(|&p| p < num_c).collect();
    let is_sl: Vec<bool> = second.iter().map(|&p| p < num_c).collect();

    // Base elevation.
    info!("Generating topography...");
    let base_raw: Vec<f32> = is_cl.iter().map(|&l| if l { 0.55 } else { 0.10 }).collect();
    let base_elev = gaussian_blur(&base_raw, mw, mh, 25.0);

    // Tectonic feature noises.
    let mtn_n = fractal_noise(mw, mh, 70, 5, 0.5, &mut rng);
    let arc_n = fractal_noise(mw, mh, 60, 4, 0.5, &mut rng);
    let crch_n = fractal_noise(mw, mh, 80, 6, 0.5, &mut rng);
    let isl_n = fractal_noise(mw, mh, 250, 4, 0.5, &mut rng);

    // Low frequency mask to break the webs into distinct ranges.
    let mask_n = fractal_noise(mw, mh, 200, 3, 0.5, &mut rng);
    // High frequency noise for carving thin, realistic valleys.
    let valley_n = fractal_noise(mw, mh, 40, 5, 0.5, &mut rng);

    // Final elevation.
    let final_elev: Vec<f32> = (0 .. n)
        .map(|i| {
            let (y, x) = (i / mw, i % mw);

            let ridge = (1.0 - (mtn_n[i] - 0.5).abs() * 2.0).powi(3);
            let arc_pk = ((arc_n[i] - 0.60) * 4.0).clamp(0.0, 1.0);
            let crunch = (crch_n[i] - 0.5) * 0.15 + (isl_n[i] - 0.5) * 0.30;

            // Only allow island arcs in active tectonic zones defined by low frequency island noise
            let active_zone = ((isl_n[i] - 0.55) * 5.0).clamp(0.0, 1.0);
            let ia =
                if !is_cl[i] && !is_sl[i] { bnd[i] * arc_pk * active_zone * 0.55 } else { 0.0 };
            let cm = if is_cl[i] ^ is_sl[i] { bnd[i] * ridge * 0.40 } else { 0.0 };
            let cc = if is_cl[i] && is_sl[i] { bnd[i] * ridge * 0.55 } else { 0.0 };

            let dx = (x as f32 / mw as f32 - 0.5).abs() * 2.0;
            let dy = (y as f32 / mh as f32 - 0.5).abs() * 2.0;
            let ep = (((dx * dx + dy * dy).sqrt() - 0.85) * 6.0)
                .clamp(0.0, 1.0)
                .powi(2)
                * 2.0;

            let raw_e = base_elev[i] + ia + cm + cc + crunch - ep;

            if raw_e > 0.40
            {
                let overage = raw_e - 0.40;

                let mask = ((mask_n[i] - 0.4) * 3.0).clamp(0.0, 1.0);
                let valley = (1.0 - (valley_n[i] - 0.5).abs() * 2.0).powi(4);
                let mut new_overage = overage * mask;
                new_overage = (new_overage - (valley * 0.10)).max(0.0);

                return 0.40 + new_overage;
            }

            return raw_e;
        })
        .collect();

    // Land/water thresholds.
    const WL: f32 = 0.35;
    info!("Enforcing 8% total elevated terrain...");
    let land_e: Vec<f32> = final_elev.iter().filter(|&&e| e >= WL).copied().collect();
    let (ht, mt) = if !land_e.is_empty()
    {
        (percentile(&land_e, 92.0), percentile(&land_e, 96.0))
    }
    else
    {
        (0.9, 1.0)
    };

    // Climate noises.
    info!("Simulating weather patterns...");
    let moist = fractal_noise(mw, mh, 200, 5, 0.5, &mut rng);
    let shore_n = fractal_noise(mw, mh, 20, 4, 0.5, &mut rng);

    // Distance to water + connected-component labelling.
    let wmask: Vec<bool> = final_elev.iter().map(|&e| e < WL).collect();
    let dtw = bfs_dt(&wmask, mw, mh);
    let (lbl_w, wsize) = label_comp(&wmask, mw, mh);

    let ocean_lbl = lbl_w[0];

    let mut lbl_pix: Vec<Vec<usize>> = vec![Vec::new(); wsize.len()];
    for (i, &l) in lbl_w.iter().enumerate()
    {
        if l > 0 && l != ocean_lbl && (l as usize) < lbl_pix.len()
        {
            lbl_pix[l as usize].push(i);
        }
    }

    // River carving.
    info!("Carving continental rivers...");
    let mtn_mask: Vec<bool> = final_elev.iter().map(|&e| e >= mt).collect();

    let mut hill_starts: Vec<(usize, usize)> = (0 .. n)
        .filter(|&i| final_elev[i] >= ht && final_elev[i] < mt && dtw[i] > 20.0)
        .map(|i| (i / mw, i % mw))
        .collect();
    shuffle(&mut hill_starts, &mut rng);

    info!(
        "Found {} potential hill starts for rivers (dtw > 20.0, elev between ht and mt)",
        hill_starts.len()
    );

    let mut is_riv = vec![false; n];
    let mut flow_m = vec![0f32; n];
    let mut lake_in = vec![0f32; wsize.len()];

    info!(" - Processing Mountain Streams...");
    let mut river_sources: Vec<(usize, usize)> = Vec::new();
    let mut carve_failures = 0usize;
    let mut astar = AStarData::new(n);

    for &(sy, sx) in &hill_starts
    {
        let mut too_close = false;
        for &(osy, osx) in &river_sources
        {
            let dy = sy as i32 - osy as i32;
            let dx = sx as i32 - osx as i32;
            if dy * dy + dx * dx < 4000
            // roughly 63 tiles apart.
            {
                too_close = true;
                break;
            }
        }

        if too_close
        {
            continue;
        }

        if !is_riv[sy * mw + sx]
        {
            let res = carve_river(
                mw,
                mh,
                &final_elev,
                &wmask,
                &mut is_riv,
                &mut flow_m,
                &lbl_w,
                &wsize,
                &dtw,
                &mut lake_in,
                ocean_lbl,
                &mut rng,
                &mut astar,
                sy,
                sx,
                1.0,
                None,
            );
            if res.is_none()
            {
                carve_failures += 1;
            }
            river_sources.push((sy, sx));
        }
    }
    info!(
        "Spawned {} rivers from mountain streams ({} carve failures).",
        river_sources.len(),
        carve_failures
    );

    info!(" - Processing Lake Overflows...");
    let mut oflow: VecDeque<(u32, f32)> = VecDeque::new();
    for l in 1 .. lake_in.len()
    {
        if lake_in[l] > 0.0 && wsize.get(l).copied().unwrap_or(0) < 15_000
        {
            oflow.push_back((l as u32, lake_in[l]));
        }
    }

    while let Some((lbl, inf)) = oflow.pop_front()
    {
        let mut best: Option<(usize, usize)> = None;
        let mut low_e = f32::MAX;

        if let Some(pixels) = lbl_pix.get(lbl as usize)
        {
            for &pi in pixels
            {
                let (ly, lx) = (pi / mw, pi % mw);
                for (dy, dx) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)]
                {
                    let ny = ly as i32 + dy;
                    let nx = lx as i32 + dx;
                    if ny < 0 || ny >= mh as i32 || nx < 0 || nx >= mw as i32
                    {
                        continue;
                    }
                    let ni = ny as usize * mw + nx as usize;
                    if !wmask[ni] && !is_riv[ni] && final_elev[ni] < low_e
                    {
                        low_e = final_elev[ni];
                        best = Some((ny as usize, nx as usize));
                    }
                }
            }
        }

        if let Some((ey, ex)) = best
        {
            if let Some(tl) = carve_river(
                mw,
                mh,
                &final_elev,
                &wmask,
                &mut is_riv,
                &mut flow_m,
                &lbl_w,
                &wsize,
                &dtw,
                &mut lake_in,
                ocean_lbl,
                &mut rng,
                &mut astar,
                ey,
                ex,
                inf,
                Some(lbl),
            )
            {
                if tl != ocean_lbl
                {
                    let sz = wsize.get(tl as usize).copied().unwrap_or(u32::MAX);
                    if sz < 15_000
                    {
                        oflow.push_back((tl, inf));
                    }
                }
            }
        }
    }

    // River width expansion.
    info!("Applying Flow Volumes to River Width...");
    let mut exp_riv = vec![false; n];
    for y in 0 .. mh
    {
        for x in 0 .. mw
        {
            let i = y * mw + x;
            if !is_riv[i]
            {
                continue;
            }
            exp_riv[i] = true;
            let fw = if flow_m[i] >= 10.0
            {
                3
            }
            else if flow_m[i] >= 4.0
            {
                2
            }
            else
            {
                1
            };

            if fw >= 2
            {
                for (dy, dx) in [(0i32, 1i32), (1, 0), (0, -1), (-1, 0)]
                {
                    let (ny, nx) = (y as i32 + dy, x as i32 + dx);
                    if ny >= 0 && ny < mh as i32 && nx >= 0 && nx < mw as i32
                    {
                        exp_riv[ny as usize * mw + nx as usize] = true;
                    }
                }
            }
            if fw == 3
            {
                for (dy, dx) in [(1i32, 1i32), (-1, -1), (1, -1), (-1, 1)]
                {
                    let (ny, nx) = (y as i32 + dy, x as i32 + dx);
                    if ny >= 0 && ny < mh as i32 && nx >= 0 && nx < mw as i32
                    {
                        exp_riv[ny as usize * mw + nx as usize] = true;
                    }
                }
            }
        }
    }

    // Final biome distances.
    info!("Populating Final Biomes...");
    let dtr = bfs_dt(&exp_riv, mw, mh);
    let dtaw: Vec<f32> = dtw
        .iter()
        .zip(dtr.iter())
        .map(|(&a, &b)| a.min(b))
        .collect();
    let dtm = bfs_dt(&mtn_mask, mw, mh);

    // Tile assignment.
    let mut tiles = vec![TileType::Ocean; n];
    tiles.par_iter_mut().enumerate().for_each(|(i, t)| {
        let (y, x) = (i / mw, i % mw);
        let e = final_elev[i];
        let m = moist[i];

        if e < WL
        {
            *t = if e < WL - 0.20
            {
                TileType::Ocean
            }
            else if e < WL - 0.08
            {
                TileType::DeepWater
            }
            else
            {
                TileType::ShallowWater
            };
            return;
        }
        if exp_riv[i]
        {
            *t = TileType::ShallowWater;
            return;
        }

        if e >= mt
        {
            *t = TileType::Mountain;
            return;
        }
        if e >= ht
        {
            *t = TileType::Hill;
            return;
        }

        if dtw[i] <= 2.0
        {
            let (mut bl, mut bs) = (0u32, 0u32);
            for dy in -2i32 ..= 2
            {
                for dx in -2i32 ..= 2
                {
                    let ny = y as i32 + dy;
                    let nx = x as i32 + dx;
                    if ny < 0 || ny >= mh as i32 || nx < 0 || nx >= mw as i32
                    {
                        continue;
                    }
                    let nl = lbl_w[ny as usize * mw + nx as usize];
                    if nl == 0
                    {
                        continue;
                    }
                    let ns = wsize.get(nl as usize).copied().unwrap_or(0);
                    if ns > bs
                    {
                        bs = ns;
                        bl = nl;
                    }
                }
            }
            let (sc, gc) = if bl == 0 || bl == ocean_lbl
            {
                (0.70f32, 0.25f32)
            }
            else if bs < 40
            {
                (0.00, 0.80)
            }
            else if bs < 150
            {
                (0.30, 0.60)
            }
            else
            {
                (0.60, 0.30)
            };
            let sn = shore_n[i];
            *t = if sn < sc
            {
                TileType::Sand
            }
            else if sn < sc + gc
            {
                TileType::PlainGrass
            }
            else
            {
                TileType::Hill
            };
            return;
        }

        *t = if m < 0.22 && dtaw[i] > 25.0 && dtm[i] > 12.0
        {
            TileType::Sand
        }
        else if m > 0.55
        {
            TileType::ForestGrass
        }
        else
        {
            TileType::PlainGrass
        };
    });

    info!("Generating per-tile variations...");
    let variations: Vec<f32> = (0 .. n).map(|_| rng.random::<f32>()).collect();

    info!("World generation complete.");

    return MapData {
        tile_size: TILE_SIZE,
        chunk_size: CHUNK_SIZE,
        tiles,
        width,
        height,
        chunks_x: MAP_WIDTH,
        chunks_y: MAP_HEIGHT,
        dirty_chunks: vec![false; (MAP_WIDTH * MAP_HEIGHT) as usize],
        macro_dirty_chunks: vec![false; (MAP_WIDTH * MAP_HEIGHT) as usize],
        variations,
    };
}

pub struct MapGenPlugin;

impl Plugin for MapGenPlugin
{
    fn build(&self, app: &mut App)
    {
        let map_data = generate_map();
        app.insert_resource(map_data);
    }
}
