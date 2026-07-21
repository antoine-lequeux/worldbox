use bevy::{asset::RenderAssetUsages, prelude::Image};

pub fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32)
{
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let mut h = 0.0;
    if delta > 0.0
    {
        if max == r
        {
            h = 60.0 * (((g - b) / delta) % 6.0);
        }
        else if max == g
        {
            h = 60.0 * (((b - r) / delta) + 2.0);
        }
        else if max == b
        {
            h = 60.0 * (((r - g) / delta) + 4.0);
        }
    }

    if h < 0.0
    {
        h += 360.0;
    }

    let s = if max == 0.0 { 0.0 } else { delta / max };
    let v = max;

    return (h, s, v);
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32)
{
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r_prime, g_prime, b_prime) = if h >= 0.0 && h < 60.0
    {
        (c, x, 0.0)
    }
    else if h >= 60.0 && h < 120.0
    {
        (x, c, 0.0)
    }
    else if h >= 120.0 && h < 180.0
    {
        (0.0, c, x)
    }
    else if h >= 180.0 && h < 240.0
    {
        (0.0, x, c)
    }
    else if h >= 240.0 && h < 300.0
    {
        (x, 0.0, c)
    }
    else
    {
        (c, 0.0, x)
    };

    return (r_prime + m, g_prime + m, b_prime + m);
}

pub fn colorize_image(base: &Image, target_color: [u8; 3]) -> Image
{
    let mut new_image = base.clone();

    new_image.asset_usage = RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD;

    let (target_h, target_s, target_v) = rgb_to_hsv(
        target_color[0] as f32 / 255.0,
        target_color[1] as f32 / 255.0,
        target_color[2] as f32 / 255.0,
    );

    if let Some(data) = &mut new_image.data
    {
        for i in (0 .. data.len()).step_by(4)
        {
            if i + 3 >= data.len()
            {
                break;
            }

            let r = data[i] as f32 / 255.0;
            let g = data[i + 1] as f32 / 255.0;
            let b = data[i + 2] as f32 / 255.0;
            let a = data[i + 3];

            if a == 0
            {
                continue;
            }

            let (h, s, v) = rgb_to_hsv(r, g, b);

            // Purple pixels have a hue of 300.
            if h >= 295.0 && h <= 305.0
            {
                let new_h = target_h;
                let new_s = (s + target_s) / 2.0;
                let new_v = (v + target_v) / 2.0;

                let (new_r, new_g, new_b) = hsv_to_rgb(new_h, new_s, new_v);

                data[i] = (new_r * 255.0).clamp(0.0, 255.0) as u8;
                data[i + 1] = (new_g * 255.0).clamp(0.0, 255.0) as u8;
                data[i + 2] = (new_b * 255.0).clamp(0.0, 255.0) as u8;
            }
        }
    }

    return new_image;
}
