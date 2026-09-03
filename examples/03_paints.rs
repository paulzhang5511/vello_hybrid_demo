//! 教程 03：画笔与描边样式
//!
//! 本教程展示 vello_hybrid 的画笔（Paint）和描边（Stroke）系统：
//! 1. 纯色画笔 Solid（Color::from_rgb8 / from_rgba8）
//! 2. 半透明叠加效果
//! 3. 描边线宽对比
//! 4. 端点样式 Cap::Butt/Round/Square
//! 5. 连接样式 Join::Miter/Round/Bevel
//! 6. 虚线模式 dash_pattern
//! 7. 填充+描边组合
//!
//! 运行：cargo run --example 03_paints
//! 输出：03_paints.png

mod common;

use vello_hybrid::Scene;
use kurbo::{BezPath, Cap, Join, Rect, Shape, Stroke};
use peniko::Color;

fn main() -> Result<(), String> {
    const WIDTH: u16 = 800;
    const HEIGHT: u16 = 600;

    let mut scene = Scene::new(WIDTH, HEIGHT);

    // 背景
    scene.set_paint(Color::from_rgb8(250, 250, 250));
    scene.fill_rect(&Rect::new(0.0, 0.0, WIDTH as f64, HEIGHT as f64));

    // ══════════════════════════════════════════════════════════════
    // 第 1 部分：纯色与半透明叠加
    // ══════════════════════════════════════════════════════════════

    // 不透明红色
    scene.set_paint(Color::from_rgb8(220, 60, 60));
    scene.fill_rect(&Rect::new(30.0, 30.0, 170.0, 130.0));

    // 半透明红色（alpha = 128/255 ≈ 50%）
    scene.set_paint(Color::from_rgba8(220, 60, 60, 128));
    scene.fill_rect(&Rect::new(100.0, 80.0, 240.0, 180.0));

    // 蓝色叠加在上面，展示混合效果
    scene.set_paint(Color::from_rgba8(60, 100, 220, 128));
    scene.fill_rect(&Rect::new(170.0, 30.0, 310.0, 130.0));

    // ══════════════════════════════════════════════════════════════
    // 第 2 部分：描边线宽对比
    // ══════════════════════════════════════════════════════════════

    scene.set_paint(Color::from_rgb8(40, 40, 40));
    let widths = [1.0, 3.0, 6.0, 10.0];
    for (i, &w) in widths.iter().enumerate() {
        scene.set_stroke(Stroke::new(w));
        scene.stroke_rect(&Rect::new(
            30.0 + (i as f64) * 90.0,
            230.0,
            100.0 + (i as f64) * 90.0,
            290.0,
        ));
    }

    // ══════════════════════════════════════════════════════════════
    // 第 3 部分：端点样式 (Cap)
    // ══════════════════════════════════════════════════════════════
    // Butt: 平端（默认），Round: 圆端，Square: 方端（延伸半个线宽）

    let cap_y = [340.0, 380.0, 420.0];
    let cap_styles = [
        (Color::from_rgb8(220, 60, 60), Cap::Butt),
        (Color::from_rgb8(60, 180, 80), Cap::Round),
        (Color::from_rgb8(60, 100, 220), Cap::Square),
    ];

    for (i, (color, cap)) in cap_styles.iter().enumerate() {
        scene.set_paint(*color);
        scene.set_stroke(Stroke::new(12.0).with_caps(*cap));
        let mut line = BezPath::new();
        line.move_to((40.0, cap_y[i]));
        line.line_to((180.0, cap_y[i]));
        scene.stroke_path(&line);
    }

    // ══════════════════════════════════════════════════════════════
    // 第 4 部分：连接样式 (Join)
    // ══════════════════════════════════════════════════════════════
    // Miter: 斜接（默认），Round: 圆角，Bevel: 斜切

    let join_colors = [
        (Color::from_rgb8(220, 100, 50), Join::Miter),
        (Color::from_rgb8(50, 180, 120), Join::Round),
        (Color::from_rgb8(100, 80, 200), Join::Bevel),
    ];

    for (i, (color, join)) in join_colors.iter().enumerate() {
        let x_offset = 230.0 + (i as f64) * 170.0;
        scene.set_paint(*color);
        scene.set_stroke(Stroke::new(10.0).with_join(*join).with_caps(Cap::Round));

        let mut path = BezPath::new();
        path.move_to((x_offset, 330.0));
        path.line_to((x_offset + 60.0, 400.0));
        path.line_to((x_offset + 120.0, 330.0));
        scene.stroke_path(&path);
    }

    // ══════════════════════════════════════════════════════════════
    // 第 5 部分：虚线样式
    // ══════════════════════════════════════════════════════════════

    scene.set_paint(Color::from_rgb8(40, 40, 40));

    // 简单虚线
    let mut dash1 = Stroke::new(3.0);
    dash1.dash_pattern = smallvec::smallvec![10.0, 5.0];
    scene.set_stroke(dash1);
    scene.stroke_rect(&Rect::new(30.0, 470.0, 200.0, 510.0));

    // 点线
    let mut dash2 = Stroke::new(3.0).with_caps(Cap::Round);
    dash2.dash_pattern = smallvec::smallvec![2.0, 8.0];
    scene.set_stroke(dash2);
    scene.stroke_rect(&Rect::new(220.0, 470.0, 390.0, 510.0));

    // 长短交替虚线
    let mut dash3 = Stroke::new(3.0);
    dash3.dash_pattern = smallvec::smallvec![15.0, 5.0, 3.0, 5.0];
    dash3.dash_offset = 0.0;
    scene.set_stroke(dash3);
    scene.stroke_rect(&Rect::new(410.0, 470.0, 580.0, 510.0));

    // ══════════════════════════════════════════════════════════════
    // 第 6 部分：填充+描边组合
    // ══════════════════════════════════════════════════════════════

    // 先填充，再描边
    let combo_circle = kurbo::Circle::new((680.0, 490.0), 40.0).to_path(0.01);

    scene.set_paint(Color::from_rgb8(255, 220, 100));
    scene.fill_path(&combo_circle);

    scene.set_paint(Color::from_rgb8(180, 120, 20));
    scene.set_stroke(Stroke::new(4.0));
    scene.stroke_path(&combo_circle);

    // 内部小圆点
    scene.set_paint(Color::from_rgb8(180, 120, 20));
    let dot1 = kurbo::Circle::new((665.0, 480.0), 5.0).to_path(0.01);
    let dot2 = kurbo::Circle::new((695.0, 480.0), 5.0).to_path(0.01);
    scene.fill_path(&dot1);
    scene.fill_path(&dot2);

    // 嘴巴
    scene.set_stroke(Stroke::new(3.0).with_caps(Cap::Round));
    let mut mouth = BezPath::new();
    mouth.move_to((668.0, 500.0));
    mouth.quad_to((680.0, 512.0), (692.0, 500.0));
    scene.stroke_path(&mouth);

    // ── 渲染 ─────────────────────────────────────────────────────
    common::render_scene_to_png(&scene, WIDTH as u32, HEIGHT as u32, "03_paints.png")?;

    Ok(())
}
