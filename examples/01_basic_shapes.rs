//! 教程 01：基础形状绘制
//!
//! 本教程展示 vello_hybrid 的核心绘制流程：
//! 1. 创建 Scene（CPU 端渲染上下文）
//! 2. 设置画笔（纯色）
//! 3. 绘制矩形、圆形、自定义贝塞尔路径
//! 4. 填充与描边
//!
//! 运行：cargo run --example 01_basic_shapes
//! 输出：01_basic_shapes.png

mod common;

use vello_hybrid::Scene;
use kurbo::{BezPath, Rect, Shape, Stroke};
use peniko::{Color, Fill};

fn main() -> Result<(), String> {
    const WIDTH: u16 = 800;
    const HEIGHT: u16 = 600;

    // ── 第 1 步：创建 Scene ──────────────────────────────────────
    // Scene 是 vello_hybrid 的 CPU 端渲染上下文，负责路径处理和几何构建。
    // 注意：宽高使用 u16，这是 vello_hybrid 的设计选择。
    let mut scene = Scene::new(WIDTH, HEIGHT);

    // ── 第 2 步：绘制红色矩形（填充） ─────────────────────────────
    // set_paint 设置后续绘制操作使用的画笔。
    // Color::from_rgb8 接受 0-255 的 RGB 值。
    scene.set_paint(Color::from_rgb8(220, 60, 60));
    scene.set_fill_rule(Fill::NonZero);
    // fill_rect 直接接受 kurbo::Rect，无需手动构建路径
    scene.fill_rect(&Rect::new(50.0, 50.0, 250.0, 200.0));

    // ── 第 3 步：绘制蓝色圆形（填充） ─────────────────────────────
    // vello_hybrid 没有直接的 fill_circle 方法，需要将 kurbo::Circle
    // 转换为 BezPath 后使用 fill_path。
    scene.set_paint(Color::from_rgb8(60, 100, 220));
    let circle_path = kurbo::Circle::new((400.0, 130.0), 80.0).to_path(0.01);
    scene.fill_path(&circle_path);

    // ── 第 4 步：绘制绿色圆角矩形（描边） ─────────────────────────
    // set_stroke 设置描边参数（线宽、端点、连接等）。
    // stroke_path / stroke_rect 执行描边绘制。
    scene.set_paint(Color::from_rgb8(40, 180, 90));
    scene.set_stroke(Stroke::new(6.0));
    let rounded_rect = kurbo::RoundedRect::new(
        550.0, 50.0, 750.0, 200.0, 20.0,
    ).to_path(0.01);
    scene.stroke_path(&rounded_rect);

    // ── 第 5 步：绘制自定义贝塞尔曲线路径（填充 + 描边） ──────────
    // 手动构建一个心形路径，展示 BezPath 的灵活性。
    scene.set_paint(Color::from_rgba8(230, 80, 140, 220));
    let heart = build_heart_path(180.0, 400.0, 100.0);
    scene.fill_path(&heart);

    // 心形描边
    scene.set_paint(Color::from_rgb8(180, 30, 80));
    scene.set_stroke(Stroke::new(3.0));
    scene.stroke_path(&heart);

    // ── 第 6 步：绘制星形路径（填充） ─────────────────────────────
    scene.set_paint(Color::from_rgb8(255, 200, 40));
    let star = build_star_path(500.0, 420.0, 90.0, 45.0, 5);
    scene.fill_path(&star);

    // ── 第 7 步：绘制虚线椭圆（描边） ─────────────────────────────
    scene.set_paint(Color::from_rgb8(120, 80, 200));
    let mut dash_stroke = Stroke::new(4.0);
    dash_stroke.dash_pattern = smallvec::smallvec![12.0, 8.0];
    dash_stroke.dash_offset = 0.0;
    scene.set_stroke(dash_stroke);
    let ellipse = kurbo::Ellipse::new((680.0, 420.0), (70.0, 50.0), 0.0).to_path(0.01);
    scene.stroke_path(&ellipse);

    // ── 第 8 步：渲染到 PNG ──────────────────────────────────────
    common::render_scene_to_png(&scene, WIDTH as u32, HEIGHT as u32, "01_basic_shapes.png")?;

    Ok(())
}

/// 构建心形贝塞尔路径。
///
/// (cx, cy) 为心形底部中心点，size 控制整体大小。
fn build_heart_path(cx: f64, cy: f64, size: f64) -> BezPath {
    let mut path = BezPath::new();
    let s = size / 16.0; // 归一化系数

    path.move_to((cx, cy - 4.0 * s));
    path.curve_to(
        (cx, cy - 8.0 * s),
        (cx - 8.0 * s, cy - 8.0 * s),
        (cx - 8.0 * s, cy - 4.0 * s),
    );
    path.curve_to(
        (cx - 8.0 * s, cy),
        (cx, cy + 4.0 * s),
        (cx, cy + 8.0 * s),
    );
    path.curve_to(
        (cx, cy + 4.0 * s),
        (cx + 8.0 * s, cy),
        (cx + 8.0 * s, cy - 4.0 * s),
    );
    path.curve_to(
        (cx + 8.0 * s, cy - 8.0 * s),
        (cx, cy - 8.0 * s),
        (cx, cy - 4.0 * s),
    );
    path.close_path();
    path
}

/// 构建星形贝塞尔路径。
///
/// (cx, cy) 为中心，outer_r 为外半径，inner_r 为内半径，points 为角数。
fn build_star_path(cx: f64, cy: f64, outer_r: f64, inner_r: f64, points: u32) -> BezPath {
    let mut path = BezPath::new();
    let total_points = points * 2;
    for i in 0..total_points {
        let angle = (i as f64) * std::f64::consts::PI / (points as f64) - std::f64::consts::PI / 2.0;
        let r = if i % 2 == 0 { outer_r } else { inner_r };
        let x = cx + r * angle.cos();
        let y = cy + r * angle.sin();
        if i == 0 {
            path.move_to((x, y));
        } else {
            path.line_to((x, y));
        }
    }
    path.close_path();
    path
}
