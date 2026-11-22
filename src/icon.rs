//! The program icon, drawn in code: an ape on a red girder with a ladder and a rolling barrel on
//! a dark blue square. Used for the window icon (`native.rs`), through `build.rs`, which includes
//! this file, for the icon resource of the executable, and (`rgba_maskable`, the `web/*.png`
//! files) for the icons of the installable web app.

/// Straight-alpha RGBA color.
type Rgba = [f32; 4];

const GIRDER: Rgba = [214.0, 60.0, 60.0, 255.0];
const LADDER: Rgba = [90.0, 200.0, 240.0, 255.0];
const BARREL: Rgba = [240.0, 150.0, 30.0, 255.0];
const APE: Rgba = [150.0, 90.0, 200.0, 255.0];
const APE_FACE: Rgba = [205.0, 165.0, 235.0, 255.0];
const EYE: Rgba = [20.0, 20.0, 40.0, 255.0];

/// Inside a rounded rectangle (x0, y0, x1, y1, corner radius); coordinates 0..1.
fn in_rounded(x: f32, y: f32, (x0, y0, x1, y1, r): (f32, f32, f32, f32, f32)) -> bool {
    if x < x0 || x > x1 || y < y0 || y > y1 {
        return false;
    }
    let cx = x.clamp(x0 + r, x1 - r);
    let cy = y.clamp(y0 + r, y1 - r);
    (x - cx).powi(2) + (y - cy).powi(2) <= r * r
}

/// Inside a circle (center x, center y, radius).
fn in_circle(x: f32, y: f32, (cx, cy, r): (f32, f32, f32)) -> bool {
    (x - cx).powi(2) + (y - cy).powi(2) <= r * r
}

/// The picture on the background: an ape on a girder, a ladder and a rolling barrel. Within
/// 0.12..0.88 on both axes, so it also fits the safe circle of a maskable icon once shrunk.
fn art(x: f32, y: f32) -> Option<Rgba> {
    // The ape: head with a lighter face and two eyes, body below it on the upper girder.
    if in_circle(x, y, (0.62, 0.20, 0.016)) || in_circle(x, y, (0.72, 0.20, 0.016)) {
        return Some(EYE);
    }
    if in_rounded(x, y, (0.57, 0.17, 0.77, 0.30, 0.04)) {
        return Some(APE_FACE);
    }
    if in_rounded(x, y, (0.52, 0.12, 0.82, 0.40, 0.07)) {
        return Some(APE);
    }
    // The barrel rolling on the upper girder (a band across its middle).
    if in_circle(x, y, (0.27, 0.335, 0.065)) {
        return Some(if (y - 0.335).abs() < 0.012 {
            GIRDER
        } else {
            BARREL
        });
    }
    // Upper and lower girder.
    if in_rounded(x, y, (0.12, 0.40, 0.88, 0.50, 0.025))
        || in_rounded(x, y, (0.12, 0.78, 0.88, 0.88, 0.025))
    {
        return Some(GIRDER);
    }
    // Ladder between the girders: two rails and three rungs.
    let rail = |x0: f32| in_rounded(x, y, (x0, 0.50, x0 + 0.03, 0.78, 0.01));
    if rail(0.55) || rail(0.70) {
        return Some(LADDER);
    }
    if (0.55..0.73).contains(&x) && [0.57, 0.64, 0.71].iter().any(|ry| (y - ry).abs() < 0.013) {
        return Some(LADDER);
    }
    None
}

/// Color of one point of the icon (coordinates 0..1). The regular icon is a rounded square on a
/// transparent background; the maskable one fills the whole square (the platform cuts the shape)
/// and keeps the picture inside the central 80 % circle.
fn sample(x: f32, y: f32, maskable: bool) -> Rgba {
    if !maskable && !in_rounded(x, y, (0.03, 0.03, 0.97, 0.97, 0.2)) {
        return [0.0; 4];
    }
    let (ax, ay) = if maskable {
        ((x - 0.5) / 0.7 + 0.5, (y - 0.5) / 0.7 + 0.5)
    } else {
        (x, y)
    };
    if let Some(color) = art(ax, ay) {
        return color;
    }
    // Background: vertical gradient from a dark blue to almost black.
    let top = [34.0, 34.0, 84.0];
    let bottom = [8.0, 8.0, 20.0];
    let mix = |i: usize| top[i] + (bottom[i] - top[i]) * y;
    [mix(0), mix(1), mix(2), 255.0]
}

/// The icon as `size` × `size` RGBA pixels (straight alpha, rows top to bottom), anti-aliased
/// with 4 × 4 samples per pixel.
pub fn rgba(size: u32) -> Vec<u8> {
    render(size, false)
}

/// The maskable variant for the web app manifest and the iOS home-screen icon: opaque and
/// full-bleed, the picture shrunk into the safe zone.
#[cfg_attr(not(test), allow(dead_code))]
pub fn rgba_maskable(size: u32) -> Vec<u8> {
    render(size, true)
}

fn render(size: u32, maskable: bool) -> Vec<u8> {
    const N: u32 = 4;
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for py in 0..size {
        for px in 0..size {
            // Premultiplied sum of the samples.
            let mut acc = [0.0f32; 4];
            for sy in 0..N {
                for sx in 0..N {
                    let x = (px as f32 + (sx as f32 + 0.5) / N as f32) / size as f32;
                    let y = (py as f32 + (sy as f32 + 0.5) / N as f32) / size as f32;
                    let c = sample(x, y, maskable);
                    let a = c[3] / 255.0;
                    for i in 0..3 {
                        acc[i] += c[i] * a;
                    }
                    acc[3] += c[3];
                }
            }
            let n = (N * N) as f32;
            let a = acc[3] / n;
            for v in &acc[..3] {
                let c = if a > 0.0 { v / n / (a / 255.0) } else { 0.0 };
                out.push(c.round().clamp(0.0, 255.0) as u8);
            }
            out.push(a.round() as u8);
        }
    }
    out
}

/// A Windows `.ico` file with the icon in the given sizes (32-bit BMP images).
#[allow(dead_code)] // used by build.rs
pub fn ico(sizes: &[u32]) -> Vec<u8> {
    let images: Vec<Vec<u8>> = sizes.iter().map(|&s| ico_image(s)).collect();
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (&s, img) in sizes.iter().zip(&images) {
        // 256 is written as 0.
        let dim = if s >= 256 { 0 } else { s as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(img.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += img.len() as u32;
    }
    for img in images {
        out.extend_from_slice(&img);
    }
    out
}

/// One `.ico` image: BITMAPINFOHEADER (double height for the mask), BGRA rows bottom to top and
/// an empty AND mask (the alpha channel is used).
#[allow(dead_code)]
fn ico_image(size: u32) -> Vec<u8> {
    let px = rgba(size);
    let mask_row = size.div_ceil(32) * 4;
    let mut out = Vec::new();
    for v in [40u32, size, size * 2] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(size * size * 4 + mask_row * size).to_le_bytes());
    out.extend_from_slice(&[0; 16]);
    for y in (0..size).rev() {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            out.extend_from_slice(&[px[i + 2], px[i + 1], px[i], px[i + 3]]);
        }
    }
    out.resize(out.len() + (mask_row * size) as usize, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_pixels_and_ico_layout() {
        let px = rgba(32);
        assert_eq!(px.len(), 32 * 32 * 4);
        // Transparent corner, opaque middle (the upper girder).
        assert_eq!(px[3], 0);
        let mid = ((14 * 32 + 8) * 4) as usize;
        assert_eq!(&px[mid..mid + 4], &[214, 60, 60, 255]);

        let ico = ico(&[16, 256]);
        assert_eq!(&ico[..6], &[0, 0, 1, 0, 2, 0]);
        // Second entry: 256 written as 0, offset after the first image.
        assert_eq!(ico[6 + 16], 0);
        let first_len = u32::from_le_bytes(ico[14..18].try_into().unwrap());
        assert_eq!(first_len, 40 + 16 * 16 * 4 + 4 * 16);
        let second_off = u32::from_le_bytes(ico[34..38].try_into().unwrap());
        assert_eq!(second_off, 6 + 32 + first_len);
        assert_eq!(ico.len() as u32, second_off + 40 + 256 * 256 * 4 + 32 * 256);
    }

    #[test]
    fn maskable_icon_is_opaque_and_full_bleed() {
        let px = rgba_maskable(64);
        assert!(px.chunks(4).all(|p| p[3] == 255));
        // The picture stays out of the corners (background gradient only).
        assert!(px[..3].iter().all(|&c| c < 100));
    }

    /// The PNG files of the web app (`web/`) are generated from this module; `UPDATE_ICONS=1
    /// cargo test web_icons` rewrites them, a plain run checks they are current.
    #[test]
    fn web_icons_are_current() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("web");
        let files: [(&str, u32, bool); 4] = [
            ("icon-192.png", 192, false),
            ("icon-512.png", 512, false),
            ("icon-maskable-512.png", 512, true),
            ("apple-touch-icon.png", 180, true),
        ];
        for (name, size, maskable) in files {
            let want = if maskable {
                rgba_maskable(size)
            } else {
                rgba(size)
            };
            let path = dir.join(name);
            if std::env::var_os("UPDATE_ICONS").is_some() {
                let file = std::fs::File::create(&path).unwrap();
                let mut enc = png::Encoder::new(std::io::BufWriter::new(file), size, size);
                enc.set_color(png::ColorType::Rgba);
                enc.set_depth(png::BitDepth::Eight);
                enc.write_header().unwrap().write_image_data(&want).unwrap();
            }
            let mut reader =
                png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap()))
                    .read_info()
                    .unwrap();
            let mut got = vec![0; reader.output_buffer_size().unwrap()];
            reader.next_frame(&mut got).unwrap();
            assert!(
                got == want,
                "{name} is outdated: run UPDATE_ICONS=1 cargo test web_icons"
            );
        }
    }
}
