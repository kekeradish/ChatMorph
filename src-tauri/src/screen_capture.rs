use screenshots::Screen;
use std::sync::Mutex;

// 缓存上一次捕获区域的平均灰度指纹
static LAST_FRAME_HASH: Mutex<Option<u64>> = Mutex::new(None);

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 截取指定屏幕区域并判断是否有显著像素变化
pub fn capture_region_if_changed(region: &CaptureRegion) -> Result<Option<String>, String> {
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let screen = screens.into_iter().next().ok_or("No screen found")?;

    // 截取指定区域
    let image_buffer = screen
        .capture_area(region.x, region.y, region.width, region.height)
        .map_err(|e| format!("Capture area failed: {}", e))?;

    let dynamic_img = image::DynamicImage::ImageRgba8(image_buffer);

    // 计算快速轻量感知哈希 (8x8 灰度平均比对)
    let small_gray = dynamic_img.resize_exact(8, 8, image::imageops::FilterType::Nearest).to_luma8();
    let pixels = small_gray.as_raw();
    let avg: u32 = pixels.iter().map(|&p| p as u32).sum::<u32>() / 64;

    let mut current_hash: u64 = 0;
    for (i, &pixel) in pixels.iter().enumerate() {
        if pixel as u32 >= avg {
            current_hash |= 1 << i;
        }
    }

    let mut last_hash = LAST_FRAME_HASH.lock().unwrap();
    if let Some(prev) = *last_hash {
        // 计算汉明距离
        let diff_bits = (prev ^ current_hash).count_ones();
        if diff_bits < 3 {
            // 画面几乎没有变化，忽略无意义识别
            return Ok(None);
        }
    }

    *last_hash = Some(current_hash);

    // 将有变化的图像编码为 PNG Base64 返回，方便前端或 OCR 引擎处理
    let mut png_bytes: Vec<u8> = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    dynamic_img
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("PNG encode failed: {}", e))?;

    let b64 = format!("data:image/png;base64,{}", base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_bytes));
    Ok(Some(b64))
}
