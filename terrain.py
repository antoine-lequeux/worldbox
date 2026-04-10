import numpy as np
from scipy.ndimage import zoom, distance_transform_edt, label, gaussian_filter
import random
import sys
import heapq

sys.stderr.reconfigure(line_buffering=True)

# Tile Types
OCEAN = 0
DEEP_WATER = 1
SHALLOW_WATER = 2
SAND = 3
PLAIN_GRASS = 4
FOREST_GRASS = 5
HILL = 6
MOUNTAIN = 7

def generate_fractal_noise(width, height, base_cell_size, octaves, persistence=0.5):
    noise = np.zeros((height, width))
    amplitude = 1.0
    max_amplitude = 0.0

    for i in range(octaves):
        cell_size = max(1, base_cell_size // (2**i))
        grid_w = max(1, width // cell_size)
        grid_h = max(1, height // cell_size)

        base_grid = np.random.random((grid_h, grid_w))
        zoom_y = height / grid_h
        zoom_x = width / grid_w
        octave_noise = zoom(base_grid, (zoom_y, zoom_x), order=2, mode='reflect')
        
        noise += octave_noise[:height, :width] * amplitude
        max_amplitude += amplitude
        amplitude *= persistence

    return noise / max_amplitude

def generate_tilemap(seed, MAP_WIDTH, MAP_HEIGHT, NUM_CONTINENTS):
    random.seed(seed)
    np.random.seed(seed & 0xFFFFFFFF)

    print(f"Starting landmasses from {NUM_CONTINENTS} centers...", file=sys.stderr)
    
    continent_centers = []
    attempts = 0
    min_dist = 450
    
    while len(continent_centers) < NUM_CONTINENTS:
        px = random.randint(250, MAP_WIDTH - 250)
        py = random.randint(250, MAP_HEIGHT - 250)
        
        if all((px - cx)**2 + (py - cy)**2 > min_dist**2 for cy, cx in continent_centers):
            continent_centers.append((py, px))
            attempts = 0
        else:
            attempts += 1
            if attempts > 50:
                min_dist -= 20
                attempts = 0

    print("Forging Ocean Rings to quarantine continents...", file=sys.stderr)
    ocean_centers = []
    
    for cy, cx in continent_centers:
        for angle in np.linspace(0, 2 * np.pi, 8, endpoint=False):
            radius = random.uniform(300, 380) 
            oy = int(cy + radius * np.sin(angle))
            ox = int(cx + radius * np.cos(angle))
            ocean_centers.append((oy, ox))

    for _ in range(25):
        ocean_centers.append((random.randint(0, MAP_HEIGHT), random.randint(0, MAP_WIDTH)))

    plate_centers = continent_centers + ocean_centers
    continent_indices = set(range(NUM_CONTINENTS))

    Y, X = np.indices((MAP_HEIGHT, MAP_WIDTH))
    warp_noise_y = (generate_fractal_noise(MAP_WIDTH, MAP_HEIGHT, 150, 4) - 0.5) * 150
    warp_noise_x = (generate_fractal_noise(MAP_WIDTH, MAP_HEIGHT, 150, 4) - 0.5) * 150
    
    Y_warped = Y + warp_noise_y
    X_warped = X + warp_noise_x

    print("Computing Tectonic fault lines...", file=sys.stderr)
    closest_plate = np.zeros((MAP_HEIGHT, MAP_WIDTH), dtype=int)
    second_closest_plate = np.zeros((MAP_HEIGHT, MAP_WIDTH), dtype=int)
    min_dist_arr = np.full((MAP_HEIGHT, MAP_WIDTH), np.inf)
    second_min_dist_arr = np.full((MAP_HEIGHT, MAP_WIDTH), np.inf)

    for i, (py, px) in enumerate(plate_centers):
        
        weight = 0.3 if i in continent_indices else 1.0 
        
        dist = ((Y_warped - py)**2 + (X_warped - px)**2) * weight
        
        mask1 = dist < min_dist_arr
        second_min_dist_arr[mask1] = min_dist_arr[mask1]
        second_closest_plate[mask1] = closest_plate[mask1]
        min_dist_arr[mask1] = dist[mask1]
        closest_plate[mask1] = i
        
        mask2 = (~mask1) & (dist < second_min_dist_arr)
        second_min_dist_arr[mask2] = dist[mask2]
        second_closest_plate[mask2] = i

    dist_closest = np.sqrt(min_dist_arr)
    dist_second = np.sqrt(second_min_dist_arr)
    boundary_mask = np.clip(1.0 - (dist_second - dist_closest) / 60.0, 0, 1)

    print("Generating topography...", file=sys.stderr)
    land_plate_mask = np.isin(closest_plate, list(continent_indices)).astype(float)
    base_elevation = land_plate_mask * 0.45 + 0.10 
    
    base_elevation = gaussian_filter(base_elevation, sigma=25)

    is_closest_land = np.isin(closest_plate, list(continent_indices))
    is_second_land = np.isin(second_closest_plate, list(continent_indices))
    
    ocean_ocean_mask = (~is_closest_land) & (~is_second_land)
    land_ocean_mask = is_closest_land ^ is_second_land

    mountain_noise = generate_fractal_noise(MAP_WIDTH, MAP_HEIGHT, 70, 5)
    mountain_ridges = (1.0 - np.abs(mountain_noise - 0.5) * 2.0) ** 3.0
    
    arc_noise = generate_fractal_noise(MAP_WIDTH, MAP_HEIGHT, 30, 4)
    arc_peaks = np.clip((arc_noise - 0.60) * 4.0, 0, 1)

    island_arcs = ocean_ocean_mask * boundary_mask * arc_peaks * 0.45
    coastal_mountains = land_ocean_mask * boundary_mask * mountain_ridges * 0.40

    crunch = (generate_fractal_noise(MAP_WIDTH, MAP_HEIGHT, 100, 6) - 0.5) * 0.35
    
    final_elevation = base_elevation + island_arcs + coastal_mountains + crunch

    dist_x = np.abs(X - MAP_WIDTH/2) / (MAP_WIDTH/2)
    dist_y = np.abs(Y - MAP_HEIGHT/2) / (MAP_HEIGHT/2)
    distance_from_center = np.sqrt(dist_x**2 + dist_y**2)
    edge_penalty = np.clip((distance_from_center - 0.85) * 6.0, 0, 1) ** 2.0
    final_elevation -= edge_penalty * 2.0 

    WATER_LEVEL = 0.35 
    
    print("Enforcing 15% Mountain/Hill ratio...", file=sys.stderr)
    land_mask = final_elevation >= WATER_LEVEL
    land_elevations = final_elevation[land_mask]
    
    if len(land_elevations) > 0:
        HILL_THRESHOLD = np.percentile(land_elevations, 85)     
        MOUNTAIN_THRESHOLD = np.percentile(land_elevations, 95) 
    else:
        HILL_THRESHOLD, MOUNTAIN_THRESHOLD = 0.9, 1.0

    print("Simulating weather patterns...", file=sys.stderr)
    moisture = generate_fractal_noise(MAP_WIDTH, MAP_HEIGHT, base_cell_size=200, octaves=5)
    shore_noise = generate_fractal_noise(MAP_WIDTH, MAP_HEIGHT, base_cell_size=20, octaves=4)

    true_water_mask = final_elevation < WATER_LEVEL
    dist_to_water = distance_transform_edt(~true_water_mask)
    labeled_water, num_water_bodies = label(true_water_mask, structure=np.ones((3,3)))
    sizes = np.bincount(labeled_water.ravel())
    ocean_label = labeled_water[0, 0]

    print("Carving continental rivers...", file=sys.stderr)
    is_river = np.zeros((MAP_HEIGHT, MAP_WIDTH), dtype=bool)
    flow_map = np.zeros((MAP_HEIGHT, MAP_WIDTH), dtype=float)
    
    hill_coords = np.argwhere((final_elevation >= HILL_THRESHOLD) & (final_elevation < MOUNTAIN_THRESHOLD))
    valid_starts = []
    
    for cy, cx in hill_coords:
        if dist_to_water[cy, cx] > 30: 
            valid_starts.append((cy, cx))

    random.shuffle(valid_starts)
    num_rivers = 90 
    rivers_spawned = 0
    lake_inflows = {lbl: 0.0 for lbl in range(1, num_water_bodies + 1)}

    def carve_river(start_y, start_x, current_flow, source_lake_lbl=None):
        pq = []
        heapq.heappush(pq, (0, start_y, start_x))
        came_from = {}
        cost_so_far = {(start_y, start_x): 0}
        target = None
        
        while pq:
            current_priority, cy, cx = heapq.heappop(pq)
            
            if true_water_mask[cy, cx]:
                if source_lake_lbl is not None and labeled_water[cy, cx] == source_lake_lbl:
                    continue
                target = (cy, cx)
                break
                
            for dy in [-1, 0, 1]:
                for dx in [-1, 0, 1]:
                    if dx == 0 and dy == 0: continue
                    ny, nx = cy + dy, cx + dx
                    
                    if 0 <= ny < MAP_HEIGHT and 0 <= nx < MAP_WIDTH:
                        if source_lake_lbl is not None and true_water_mask[ny, nx] and labeled_water[ny, nx] == source_lake_lbl:
                            continue

                        z_diff = final_elevation[ny, nx] - final_elevation[cy, cx]
                        
                        if is_river[ny, nx]:
                            step_cost = 0.1 
                        else:
                            if z_diff > 0:
                                step_cost = 10.0 + z_diff * 500.0 
                            else:
                                step_cost = 1.0 + z_diff * 10.0  
                            step_cost += random.uniform(0.5, 3.0) 
                            
                        step_cost = max(0.1, step_cost) 
                        new_cost = cost_so_far[(cy, cx)] + step_cost
                        
                        if (ny, nx) not in cost_so_far or new_cost < cost_so_far[(ny, nx)]:
                            cost_so_far[(ny, nx)] = new_cost
                            priority = new_cost + (dist_to_water[ny, nx] * 0.4)
                            heapq.heappush(pq, (priority, ny, nx))
                            came_from[(ny, nx)] = (cy, cx)
                            
        if target:
            curr = target
            path = []
            while curr != (start_y, start_x):
                path.append(curr)
                curr = came_from[curr]
            path.append((start_y, start_x))
            path.reverse()
            
            for i in range(len(path)):
                ry, rx = path[i]
                if not true_water_mask[ry, rx]:
                    is_river[ry, rx] = True
                    flow_map[ry, rx] += current_flow
                
                if i < len(path) - 1:
                    ny, nx = path[i+1]
                    if abs(ry - ny) == 1 and abs(rx - nx) == 1:
                        if not true_water_mask[ry, nx]:
                            is_river[ry, nx] = True
                            flow_map[ry, nx] += current_flow

            end_y, end_x = target
            target_label = labeled_water[end_y, end_x]
            if target_label != ocean_label and target_label != 0:
                lake_inflows[target_label] += current_flow
                return target_label
        return None

    print(" - Processing Mountain Streams...", file=sys.stderr)
    for y, x in valid_starts:
        if rivers_spawned >= num_rivers: break
        if not is_river[y, x]:
            carve_river(y, x, 1.0)
            rivers_spawned += 1

    print(" - Processing Lake Overflows...", file=sys.stderr)
    overflow_queue = []
    for lbl, inflow in lake_inflows.items():
        if inflow > 0 and sizes[lbl] < 15000:
            overflow_queue.append((lbl, inflow))

    while overflow_queue:
        lbl, inflow = overflow_queue.pop(0)
        lake_coords = np.argwhere(labeled_water == lbl)
        best_exit = None
        lowest_e = 9999

        for ly, lx in lake_coords:
            for dy in [-1, 0, 1]:
                for dx in [-1, 0, 1]:
                    ny, nx = ly + dy, lx + dx
                    if 0 <= ny < MAP_HEIGHT and 0 <= nx < MAP_WIDTH:
                        if not true_water_mask[ny, nx] and not is_river[ny, nx]:
                            if final_elevation[ny, nx] < lowest_e:
                                lowest_e = final_elevation[ny, nx]
                                best_exit = (ny, nx)

        if best_exit:
            target_lake = carve_river(best_exit[0], best_exit[1], inflow, source_lake_lbl=lbl)
            if target_lake and target_lake != ocean_label and sizes[target_lake] < 15000:
                overflow_queue.append((target_lake, inflow))

    print("Applying Flow Volumes to River Width...", file=sys.stderr)
    expanded_river_mask = np.zeros((MAP_HEIGHT, MAP_WIDTH), dtype=bool)
    for y in range(MAP_HEIGHT):
        for x in range(MAP_WIDTH):
            if is_river[y, x]:
                f = flow_map[y, x]
                expanded_river_mask[y, x] = True
                
                width = 1
                if f >= 4.0: width = 2
                if f >= 10.0: width = 3

                if width >= 2:
                    for dy, dx in [(0,1), (1,0), (0,-1), (-1,0)]:
                        if 0 <= y+dy < MAP_HEIGHT and 0 <= x+dx < MAP_WIDTH:
                            expanded_river_mask[y+dy, x+dx] = True
                if width == 3:
                    for dy, dx in [(1,1), (-1,-1), (1,-1), (-1,1)]:
                        if 0 <= y+dy < MAP_HEIGHT and 0 <= x+dx < MAP_WIDTH:
                            expanded_river_mask[y+dy, x+dx] = True

    print("Populating Final Biomes...", file=sys.stderr)
    dist_to_river = distance_transform_edt(~expanded_river_mask)
    dist_to_any_water = np.minimum(dist_to_water, dist_to_river)
    
    mountain_mask = final_elevation >= MOUNTAIN_THRESHOLD
    dist_to_mountain = distance_transform_edt(~mountain_mask)
    
    tilemap = np.zeros((MAP_HEIGHT, MAP_WIDTH), dtype=int)

    for y in range(MAP_HEIGHT):
        for x in range(MAP_WIDTH):
            e = final_elevation[y, x]
            m = moisture[y, x]
            
            if e < WATER_LEVEL:
                if e < WATER_LEVEL - 0.20:
                    tilemap[y, x] = OCEAN
                elif e < WATER_LEVEL - 0.08:
                    tilemap[y, x] = DEEP_WATER
                else:
                    tilemap[y, x] = SHALLOW_WATER
                continue
                
            if expanded_river_mask[y, x]:
                tilemap[y, x] = SHALLOW_WATER
                continue
            
            if e >= MOUNTAIN_THRESHOLD:
                tilemap[y, x] = MOUNTAIN
            elif e >= HILL_THRESHOLD:
                tilemap[y, x] = HILL
            else:
                if dist_to_water[y, x] <= 2:
                    adj_labels = []
                    for dy in [-2, -1, 0, 1, 2]:
                        for dx in [-2, -1, 0, 1, 2]:
                            ny, nx = y+dy, x+dx
                            if 0 <= ny < MAP_HEIGHT and 0 <= nx < MAP_WIDTH:
                                lbl = labeled_water[ny, nx]
                                if lbl > 0: adj_labels.append(lbl)
                    
                    if adj_labels:
                        main_water_lbl = max(adj_labels, key=lambda l: sizes[l])
                        water_size = sizes[main_water_lbl]
                        
                        if main_water_lbl == ocean_label:
                            sand_chance, grass_chance = 0.70, 0.25
                        elif water_size < 40: 
                            sand_chance, grass_chance = 0.00, 0.80
                        elif water_size < 150:
                            sand_chance, grass_chance = 0.30, 0.60
                        else:                  
                            sand_chance, grass_chance = 0.60, 0.30
                    else:
                        sand_chance, grass_chance = 0.70, 0.25
                        
                    sn = shore_noise[y, x]
                    if sn < sand_chance:
                        tilemap[y, x] = SAND
                    elif sn < sand_chance + grass_chance:
                        tilemap[y, x] = PLAIN_GRASS
                    else:
                        tilemap[y, x] = HILL
                else:
                    if m < 0.22 and dist_to_any_water[y, x] > 25 and dist_to_mountain[y, x] > 12:
                        tilemap[y, x] = SAND 
                    elif m > 0.55:
                        tilemap[y, x] = FOREST_GRASS
                    else:
                        tilemap[y, x] = PLAIN_GRASS

    return tilemap

if __name__ == "__main__":
    seed = int(sys.argv[1])
    width = int(sys.argv[2])
    height = int(sys.argv[3])
    num_continents = int(sys.argv[4])

    tilemap = generate_tilemap(seed, width, height, num_continents)

    for row in tilemap:
        print(''.join(map(str, row)))