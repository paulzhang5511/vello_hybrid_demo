//! 教程 04：图层与裁剪
//!
//! 本教程展示 vello_hybrid 的图层管理系统：
//! 1. push_clip_path / pop_clip_path —— 轻量路径裁剪
//! 2. push_clip_layer —— 离屏图层裁剪（抗锯齿边界）
//! 3. push_opacity_layer —— 整体透明度图层
//! 4. push_filter_layer —— 滤镜图层（高斯模糊）
//! 5. push_layer 通用图层（clip + opacity + filter 组合）
//!
//! 注意（vello_hybrid 0.2.0 限制）：
//! - Mask layers 尚未支持
//! - 复杂 filter graphs 不支持
//!
//! 运行：cargo run --example 04_layers_clipping
//! 输出：04_layers_clipping.png

mod common;

use vello_hybrid::Scene;
use kurbo::{BezPath, Rect, Shape, Stroke};
use peniko::Color;
use vello_common::filter_effects::{Filter, FilterFunction};

fn main() -> Result<(), String> {
    const WIDTH: u16 = 800;
    const HEIGHT: u16 = 600;

    let mut scene = Scene::new(WIDTH, HEIGHT);

    // 背景
    scene.set_paint(Color::from_rgb8(248, 248, 248));
    scene.fill_rect(&Rect::new(0.0, 0.0, WIDTH as f64, HEIGHT as f64));

    // ══════════════════════════════════════════════════════════════
    // 第 1 部分：push_clip_path —— 轻量路径裁剪
    // ══════════════════════════════════════════════════════════════
    //
    // push_clip_path 是轻量级裁剪：后续绘制被限制在路径内部。
    // 不创建离屏缓冲，性能开销小，但裁剪边界不抗锯齿。

    let clip_circle = kurbo::Circle::new((120.0, 120.0), 90.0).to_path(0.01);

    scene.push_clip_path(&clip_circle);
    // 在裁剪区域内绘制彩色条纹
    let colors = [
        Color::from_rgb8(230, 70, 70),
        Color::from_rgb8(230, 160, 50),
        Color::from_rgb8(230, 220, 60),
        Color::from_rgb8(80, 200, 90),
        Color::from_rgb8(70, 150, 220),
    ];
    for (i, &color) in colors.iter().enumerate() {
        scene.set_paint(color);
        scene.fill_rect(&Rect::new(
            30.0 + (i as f64) * 36.0,
            30.0,
            66.0 + (i as f64) * 36.0,
            210.0,
        ));
    }
    scene.pop_clip_path();

    // 裁剪区域外的参考圆（描边）
    scene.set_paint(Color::from_rgb8(100, 100, 100));
    scene.set_stroke(Stroke::new(2.0));
    scene.stroke_path(&clip_circle);

    // ══════════════════════════════════════════════════════════════
    // 第 2 部分：push_clip_layer —— 离屏图层裁剪
    // ══════════════════════════════════════════════════════════════
    //
    // push_clip_layer 创建离屏缓冲，裁剪边界抗锯齿。
    // 必须在渲染前 pop 所有 clip layer。

    let clip_rounded = kurbo::RoundedRect::new(260.0, 40.0, 460.0, 200.0, 30.0).to_path(0.01);

    scene.push_clip_layer(&clip_rounded);
    // 绘制渐变背景（用纯色模拟夜空）
    scene.set_paint(Color::from_rgb8(30, 40, 80));
    scene.fill_rect(&Rect::new(260.0, 40.0, 460.0, 200.0));
    // 绘制星星
    scene.set_paint(Color::from_rgb8(255, 255, 200));
    let star_positions = [
        (290.0, 70.0), (320.0, 120.0), (350.0, 90.0), (380.0, 150.0),
        (410.0, 80.0), (430.0, 130.0), (300.0, 170.0), (370.0, 180.0),
        (420.0, 170.0), (340.0, 140.0),
    ];
    for &(x, y) in &star_positions {
        let star = kurbo::Circle::new((x, y), 2.5).to_path(0.01);
        scene.fill_path(&star);
    }
    // 绘制月亮
    scene.set_paint(Color::from_rgb8(240, 240, 220));
    let moon = kurbo::Circle::new((400.0, 90.0), 25.0).to_path(0.01);
    scene.fill_path(&moon);
    scene.pop_layer();

    // 参考边框
    scene.set_paint(Color::from_rgb8(100, 100, 100));
    scene.set_stroke(Stroke::new(2.0));
    scene.stroke_path(&clip_rounded);

    // ══════════════════════════════════════════════════════════════
    // 第 3 部分：push_opacity_layer —— 透明度图层
    // ══════════════════════════════════════════════════════════════
    //
    // 整个图层的内容统一应用透明度，图层内部元素先合成再整体透明。
    // 这与直接给每个元素设置半透明不同：图层内元素之间不透明，
    // 图层作为一个整体与背景混合。

    // 先画背景参考框
    scene.set_paint(Color::from_rgb8(200, 200, 200));
    scene.fill_rect(&Rect::new(500.0, 40.0, 760.0, 200.0));

    // 透明度 0.5 的图层
    scene.push_opacity_layer(0.5);
    // 两个重叠的圆形，它们之间不透明（因为在同一图层内先合成）
    scene.set_paint(Color::from_rgb8(220, 60, 60));
    let c1 = kurbo::Circle::new((570.0, 120.0), 50.0).to_path(0.01);
    scene.fill_path(&c1);

    scene.set_paint(Color::from_rgb8(60, 100, 220));
    let c2 = kurbo::Circle::new((640.0, 120.0), 50.0).to_path(0.01);
    scene.fill_path(&c2);

    scene.set_paint(Color::from_rgb8(60, 180, 80));
    let c3 = kurbo::Circle::new((605.0, 170.0), 50.0).to_path(0.01);
    scene.fill_path(&c3);
    scene.pop_layer();

    // ══════════════════════════════════════════════════════════════
    // 第 4 部分：push_filter_layer —— 滤镜图层（高斯模糊）
    // ══════════════════════════════════════════════════════════════

    // 原始（无滤镜）
    scene.set_paint(Color::from_rgb8(220, 80, 80));
    scene.fill_rect(&Rect::new(30.0, 260.0, 170.0, 360.0));
    scene.set_paint(Color::from_rgb8(40, 40, 40));
    scene.fill_rect(&Rect::new(50.0, 290.0, 150.0, 330.0));

    // 高斯模糊滤镜（radius = 8.0）
    scene.push_filter_layer(Filter::from_function(FilterFunction::Blur {
        radius: 8.0,
    }));
    scene.set_paint(Color::from_rgb8(80, 150, 220));
    scene.fill_rect(&Rect::new(210.0, 260.0, 350.0, 360.0));
    scene.set_paint(Color::from_rgb8(40, 40, 40));
    scene.fill_rect(&Rect::new(230.0, 290.0, 330.0, 330.0));
    scene.pop_layer();

    // 更强的模糊（radius = 20.0）
    scene.push_filter_layer(Filter::from_function(FilterFunction::Blur {
        radius: 20.0,
    }));
    scene.set_paint(Color::from_rgb8(80, 200, 120));
    scene.fill_rect(&Rect::new(390.0, 260.0, 530.0, 360.0));
    scene.set_paint(Color::from_rgb8(40, 40, 40));
    scene.fill_rect(&Rect::new(410.0, 290.0, 510.0, 330.0));
    scene.pop_layer();

    // ══════════════════════════════════════════════════════════════
    // 第 5 部分：组合使用 —— 裁剪 + 透明度 + 滤镜
    // ══════════════════════════════════════════════════════════════

    let combo_clip = kurbo::RoundedRect::new(570.0, 260.0, 770.0, 430.0, 20.0).to_path(0.01);

    scene.push_clip_layer(&combo_clip);
    scene.push_opacity_layer(0.85);

    // 背景
    scene.set_paint(Color::from_rgb8(50, 50, 80));
    scene.fill_rect(&Rect::new(570.0, 260.0, 770.0, 430.0));

    // 山形
    scene.set_paint(Color::from_rgb8(80, 100, 140));
    let mut mountain = BezPath::new();
    mountain.move_to((570.0, 430.0));
    mountain.line_to((620.0, 330.0));
    mountain.line_to((670.0, 370.0));
    mountain.line_to((720.0, 310.0));
    mountain.line_to((770.0, 430.0));
    mountain.close_path();
    scene.fill_path(&mountain);

    // 太阳
    scene.set_paint(Color::from_rgb8(255, 200, 100));
    let sun = kurbo::Circle::new((720.0, 300.0), 25.0).to_path(0.01);
    scene.fill_path(&sun);

    scene.pop_layer(); // opacity
    scene.pop_layer(); // clip

    // 边框
    scene.set_paint(Color::from_rgb8(100, 100, 100));
    scene.set_stroke(Stroke::new(2.0));
    scene.stroke_path(&combo_clip);

    // ══════════════════════════════════════════════════════════════
    // 第 6 部分：多层嵌套裁剪
    // ══════════════════════════════════════════════════════════════

    // 外层裁剪：大圆
    let outer_clip = kurbo::Circle::new((150.0, 480.0), 80.0).to_path(0.01);
    scene.push_clip_path(&outer_clip);

    // 内层裁剪：小圆（偏移）
    let inner_clip = kurbo::Circle::new((180.0, 460.0), 50.0).to_path(0.01);
    scene.push_clip_path(&inner_clip);

    // 绘制彩色条纹（只在两个圆的交集区域可见）
    let stripe_colors = [
        Color::from_rgb8(255, 100, 100),
        Color::from_rgb8(100, 255, 100),
        Color::from_rgb8(100, 100, 255),
        Color::from_rgb8(255, 255, 100),
    ];
    for (i, &color) in stripe_colors.iter().enumerate() {
        scene.set_paint(color);
        scene.fill_rect(&Rect::new(
            70.0 + (i as f64) * 40.0,
            400.0,
            110.0 + (i as f64) * 40.0,
            560.0,
        ));
    }

    scene.pop_clip_path(); // inner
    scene.pop_clip_path(); // outer

    // 参考圆
    scene.set_paint(Color::from_rgb8(100, 100, 100));
    scene.set_stroke(Stroke::new(1.5));
    scene.stroke_path(&outer_clip);
    scene.stroke_path(&inner_clip);

    // ── 渲染 ─────────────────────────────────────────────────────
    common::render_scene_to_png(&scene, WIDTH as u32, HEIGHT as u32, "04_layers_clipping.png")?;

    Ok(())
}
