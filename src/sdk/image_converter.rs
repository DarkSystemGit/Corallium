use std::path::PathBuf;

pub fn convert_image(input_image_file: &str) -> Result<(), String> {
    let decoded = image::open(input_image_file)
        .map_err(|e| format!("Failed to decode image '{}': {}", input_image_file, e))?
        .to_rgba8();
    let (width, height) = decoded.dimensions();

    if width == 0 || height == 0 {
        return Err(format!(
            "Image '{}' has invalid dimensions {}x{}",
            input_image_file, width, height
        ));
    }

    if width > i16::MAX as u32 || height > i16::MAX as u32 {
        return Err(format!(
            "Image '{}' dimensions {}x{} exceed i16 limits (max {}x{})",
            input_image_file,
            width,
            height,
            i16::MAX,
            i16::MAX
        ));
    }

    let width_i16 = width as i16;
    let height_i16 = height as i16;
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| format!("Image '{}' pixel count overflow", input_image_file))?;
    let pixel_bytes = pixel_count
        .checked_mul(4)
        .ok_or_else(|| format!("Image '{}' output size overflow", input_image_file))?;

    let mut bytes = Vec::with_capacity(4 + pixel_bytes);
    bytes.extend_from_slice(&width_i16.to_le_bytes());
    bytes.extend_from_slice(&height_i16.to_le_bytes());

    for px in decoded.as_raw().chunks_exact(4) {
        let r = px[0] as u32;
        let g = px[1] as u32;
        let b = px[2] as u32;
        let a = px[3] as u32;
        let packed = ((r << 24) | (g << 16) | (b << 8) | a).to_le_bytes();
        bytes.extend_from_slice(&packed);
    }

    let expected_len = 4 + pixel_bytes;
    if bytes.len() != expected_len {
        return Err(format!(
            "Internal conversion error for '{}': expected {} bytes, produced {} bytes",
            input_image_file,
            expected_len,
            bytes.len()
        ));
    }

    let output_path = default_output_path(input_image_file);
    std::fs::write(&output_path, bytes).map_err(|e| {
        format!(
            "Failed to write converted output '{}': {}",
            output_path.display(),
            e
        )
    })?;
    Ok(())
}

fn default_output_path(input: &str) -> PathBuf {
    let mut out = PathBuf::from(input);
    if out.file_name().is_none() {
        return PathBuf::from("output.cbmp");
    }
    out.set_extension("cbmp");
    out
}
