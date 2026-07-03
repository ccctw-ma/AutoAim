fn main() {
    write_icon();
    tauri_build::build();
}

fn write_icon() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let icon_dir = std::path::Path::new(&manifest_dir).join("icons");
    let icon_path = icon_dir.join("icon.ico");
    let icon_png_path = icon_dir.join("icon.png");
    std::fs::create_dir_all(&icon_dir).expect("create icon dir");

    let size = 64usize;
    let center = (size as f32 - 1.0) / 2.0;
    let mut pixels = Vec::with_capacity(size * size * 4);

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let radius = (dx * dx + dy * dy).sqrt();
            let (mut r, mut g, mut b, a) = (11u8, 16u8, 32u8, 255u8);

            if (20.0..=24.0).contains(&radius) {
                (r, g, b) = (34, 211, 238);
            } else if (30.0..=32.0).contains(&radius) {
                (r, g, b) = (31, 41, 55);
            }

            if dx.abs() <= 2.0 && (radius < 12.0 || radius > 25.0) {
                (r, g, b) = (34, 211, 238);
            }
            if dy.abs() <= 2.0 && (radius < 12.0 || radius > 25.0) {
                (r, g, b) = (34, 211, 238);
            }

            let left_stem =
                (x as f32 - (22.0 + y as f32 * 0.22)).abs() <= 2.0 && (18..=47).contains(&y);
            let right_stem =
                (x as f32 - (42.0 - y as f32 * 0.22)).abs() <= 2.0 && (18..=47).contains(&y);
            let crossbar = (35..=39).contains(&y) && (26..=38).contains(&x);
            if left_stem || right_stem || crossbar {
                (r, g, b) = (248, 250, 252);
            }

            if radius <= 3.0 {
                (r, g, b) = (249, 115, 22);
            }

            pixels.extend_from_slice(&[r, g, b, a]);
        }
    }

    write_png(&icon_png_path, size as u32, size as u32, &pixels);

    let mut xor_bitmap = Vec::with_capacity(size * size * 4);
    for y in (0..size).rev() {
        let row_start = y * size * 4;
        for pixel in pixels[row_start..row_start + size * 4].chunks_exact(4) {
            xor_bitmap.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }

    let and_mask = vec![0u8; (size / 8) * size];
    let image_size = 40 + xor_bitmap.len() + and_mask.len();
    let mut data = Vec::with_capacity(22 + image_size);

    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&[size as u8, size as u8, 0, 0]);
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&32u16.to_le_bytes());
    data.extend_from_slice(&(image_size as u32).to_le_bytes());
    data.extend_from_slice(&22u32.to_le_bytes());

    data.extend_from_slice(&40u32.to_le_bytes());
    data.extend_from_slice(&(size as u32).to_le_bytes());
    data.extend_from_slice(&((size * 2) as u32).to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&32u16.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&(xor_bitmap.len() as u32).to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&xor_bitmap);
    data.extend_from_slice(&and_mask);

    std::fs::write(icon_path, data).expect("write generated icon");
}

fn write_png(path: &std::path::Path, width: u32, height: u32, rgba: &[u8]) {
    image::save_buffer(path, rgba, width, height, image::ColorType::Rgba8)
        .expect("write generated png icon");
}
