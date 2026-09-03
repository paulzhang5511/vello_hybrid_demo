//! 教程 05：文字渲染
//!
//! 本教程展示 vello_hybrid 的文字渲染核心 API：
//! 1. 加载字体文件 → FontData
//! 2. scene.glyph_run() → GlyphRunBuilder
//! 3. font_size() 设置字号
//! 4. fill_glyphs() 填充文字
//! 5. stroke_glyphs() 描边文字
//! 6. font_embolden() 合成粗体
//! 7. glyph_transform() 字形变换（斜体等）
//!
//! 注意：文字渲染需要在构建 Scene 时访问 Resources，
//! 因此流程是：创建 Renderer/Resources → 构建 Scene（含 glyph_run）→ 渲染。
//!
//! 本教程使用简化的字形布局（固定 advance width），
//! 生产环境建议使用 parley 等专业文本布局库。
//!
//! 运行：cargo run --example 05_text_rendering
//! 输出：05_text_rendering.png

mod common;

use vello_hybrid::Scene;
use kurbo::{Affine, Diagonal2, Rect, Stroke};
use peniko::{Blob, Color, FontData};
use glifo::{FontEmbolden, Glyph};

/// 尝试从多个常见路径加载系统字体。
fn load_system_font() -> Result<Vec<u8>, String> {
    let candidates = [
        // macOS 系统字体
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
        "/Library/Fonts/Arial.ttf",
        // Linux 常见字体
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
    ];

    for path in &candidates {
        if let Ok(data) = std::fs::read(path) {
            println!("使用字体: {path}");
            return Ok(data);
        }
    }
    Err(format!(
        "未找到系统字体。请在以下路径之一放置字体文件，或修改 load_system_font() 函数。\n候选路径: {:?}",
        candidates
    ))
}

/// 简化的文本布局：将字符串转换为带位置的 Glyph 列表。
///
/// 使用 skrifa 的 charmap 进行字符到 glyph_id 的映射，
/// 使用固定的 advance width（font_size * 0.6）进行水平布局。
/// 生产环境建议使用 parley 等专业文本布局库。
fn layout_text(
    font_data: &[u8],
    text: &str,
    font_size: f32,
) -> Vec<Glyph> {
    use skrifa::{FontRef, MetadataProvider};

    let font = match FontRef::new(font_data) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let cmap = font.charmap();
    let advance = font_size * 0.6; // 简化：固定 advance width

    let mut glyphs = Vec::new();
    let mut x = 0.0f32;

    for ch in text.chars() {
        if ch == '\n' {
            x = 0.0;
            continue;
        }
        if ch == ' ' {
            x += advance;
            continue;
        }
        if let Some(glyph_id) = cmap.map(ch) {
            glyphs.push(Glyph {
                id: glyph_id.to_u32(),
                x,
                y: 0.0,
            });
            x += advance;
        }
    }

    glyphs
}

fn main() -> Result<(), String> {
    const WIDTH: u16 = 800;
    const HEIGHT: u16 = 600;

    // 加载字体
    let font_bytes = load_system_font()?;
    let font_data = FontData::new(Blob::new(std::sync::Arc::new(font_bytes.clone())), 0);

    // ══════════════════════════════════════════════════════════════
    // 文字渲染需要先创建 Renderer/Resources
    // ══════════════════════════════════════════════════════════════
    let (device, queue) = common::init_wgpu()?;
    let (mut renderer, mut resources) =
        common::create_renderer(&device, WIDTH as u32, HEIGHT as u32);

    let mut scene = Scene::new(WIDTH, HEIGHT);

    // 背景
    scene.set_paint(Color::from_rgb8(252, 252, 252));
    scene.fill_rect(&Rect::new(0.0, 0.0, WIDTH as f64, HEIGHT as f64));

    // ══════════════════════════════════════════════════════════════
    // 第 1 部分：基本文字渲染（不同字号）
    // ══════════════════════════════════════════════════════════════

    let sizes = [16.0f32, 24.0, 32.0, 48.0];
    let colors = [
        Color::from_rgb8(60, 60, 60),
        Color::from_rgb8(80, 80, 120),
        Color::from_rgb8(100, 60, 60),
        Color::from_rgb8(60, 100, 80),
    ];

    let mut y = 40.0f64;
    for (i, &size) in sizes.iter().enumerate() {
        let text = format!("vello_hybrid Text {}px", size as i32);
        let glyphs = layout_text(&font_bytes, &text, size);

        scene.set_paint(colors[i]);
        scene
            .glyph_run(&mut resources, &font_data)
            .font_size(size)
            .fill_glyphs(glyphs.iter().map(|g| Glyph {
                id: g.id,
                x: g.x,
                y: g.y + y as f32,
            }));

        y += size as f64 * 1.5;
    }

    // ══════════════════════════════════════════════════════════════
    // 第 2 部分：描边文字
    // ══════════════════════════════════════════════════════════════

    let stroke_text = "Stroke Text";
    let stroke_size = 40.0f32;
    let stroke_glyphs = layout_text(&font_bytes, stroke_text, stroke_size);

    // 先填充浅色
    scene.set_paint(Color::from_rgb8(240, 240, 255));
    scene
        .glyph_run(&mut resources, &font_data)
        .font_size(stroke_size)
        .fill_glyphs(stroke_glyphs.iter().map(|g| Glyph {
            id: g.id,
            x: g.x,
            y: g.y + 280.0,
        }));

    // 再描边深色
    scene.set_paint(Color::from_rgb8(40, 40, 120));
    scene.set_stroke(Stroke::new(2.0));
    scene
        .glyph_run(&mut resources, &font_data)
        .font_size(stroke_size)
        .stroke_glyphs(stroke_glyphs.iter().map(|g| Glyph {
            id: g.id,
            x: g.x,
            y: g.y + 280.0,
        }));

    // ══════════════════════════════════════════════════════════════
    // 第 3 部分：合成粗体 (font_embolden)
    // ══════════════════════════════════════════════════════════════

    let bold_text = "Synthetic Bold";
    let bold_size = 36.0f32;
    let bold_glyphs = layout_text(&font_bytes, bold_text, bold_size);

    // 普通
    scene.set_paint(Color::from_rgb8(80, 80, 80));
    scene
        .glyph_run(&mut resources, &font_data)
        .font_size(bold_size)
        .fill_glyphs(bold_glyphs.iter().map(|g| Glyph {
            id: g.id,
            x: g.x,
            y: g.y + 360.0,
        }));

    // 合成粗体
    scene.set_paint(Color::from_rgb8(40, 40, 40));
    scene
        .glyph_run(&mut resources, &font_data)
        .font_size(bold_size)
        .font_embolden(FontEmbolden::new(Diagonal2::new(0.5, 0.0)))
        .fill_glyphs(bold_glyphs.iter().map(|g| Glyph {
            id: g.id,
            x: g.x,
            y: g.y + 410.0,
        }));

    // ══════════════════════════════════════════════════════════════
    // 第 4 部分：字形变换 (glyph_transform) —— 斜体
    // ══════════════════════════════════════════════════════════════

    let italic_text = "Italic Style";
    let italic_size = 32.0f32;
    let italic_glyphs = layout_text(&font_bytes, italic_text, italic_size);

    // 普通
    scene.set_paint(Color::from_rgb8(80, 80, 80));
    scene
        .glyph_run(&mut resources, &font_data)
        .font_size(italic_size)
        .fill_glyphs(italic_glyphs.iter().map(|g| Glyph {
            id: g.id,
            x: g.x,
            y: g.y + 490.0,
        }));

    // 斜体（水平错切 -0.3）
    scene.set_paint(Color::from_rgb8(120, 60, 120));
    scene
        .glyph_run(&mut resources, &font_data)
        .font_size(italic_size)
        .glyph_transform(Affine::new([1.0, 0.0, -0.3, 1.0, 0.0, 0.0]))
        .fill_glyphs(italic_glyphs.iter().map(|g| Glyph {
            id: g.id,
            x: g.x,
            y: g.y + 540.0,
        }));

    // ── 渲染 ─────────────────────────────────────────────────────
    common::render_with_renderer_to_png(
        &device,
        &queue,
        &mut renderer,
        &mut resources,
        &scene,
        WIDTH as u32,
        HEIGHT as u32,
        "05_text_rendering.png",
    )?;

    Ok(())
}
