//! 教程 02：变换与坐标系
//!
//! 本教程展示 vello_hybrid 的变换系统：
//! 1. set_transform —— 全局几何变换（平移、旋转、缩放、错切）
//! 2. 变换的组合与累积
//! 3. set_paint_transform —— 画笔独立变换（渐变/图像随几何变换 vs 独立变换）
//! 4. reset_transform —— 恢复单位矩阵
//!
//! 运行：cargo run --example 02_transforms
//! 输出：02_transforms.png

mod common;

use vello_hybrid::Scene;
use kurbo::{Affine, Rect, Shape, Stroke};
use peniko::Color;

fn main() -> Result<(), String> {
    const WIDTH: u16 = 800;
    const HEIGHT: u16 = 600;

    let mut scene = Scene::new(WIDTH, HEIGHT);

    // ── 背景：浅灰色 ─────────────────────────────────────────────
    scene.set_paint(Color::from_rgb8(245, 245, 245));
    scene.fill_rect(&Rect::new(0.0, 0.0, WIDTH as f64, HEIGHT as f64));

    // ══════════════════════════════════════════════════════════════
    // 第 1 部分：平移 (translate)
    // ══════════════════════════════════════════════════════════════

    // 参考矩形（原始位置，左上角）
    scene.set_paint(Color::from_rgba8(180, 180, 180, 120));
    scene.fill_rect(&Rect::new(30.0, 30.0, 130.0, 110.0));

    // 平移后的矩形：向右 160px，向下 40px
    // Affine::translate((dx, dy)) 创建平移矩阵
    scene.set_transform(Affine::translate((160.0, 40.0)));
    scene.set_paint(Color::from_rgb8(60, 140, 220));
    scene.fill_rect(&Rect::new(30.0, 30.0, 130.0, 110.0));
    scene.reset_transform(); // 恢复单位矩阵，避免影响后续绘制

    // ══════════════════════════════════════════════════════════════
    // 第 2 部分：旋转 (rotate)
    // ══════════════════════════════════════════════════════════════

    // 旋转中心在 (350, 100)，旋转 30 度
    // Affine::rotate(angle) 绕原点旋转，需要先平移到旋转中心再旋转再平移回来
    // 或使用 Affine::translate(center) * Affine::rotate(angle) * Affine::translate(-center)
    let center = (350.0, 100.0);
    let rotate_30 = Affine::translate(center)
        * Affine::rotate(std::f64::consts::PI / 6.0) // 30度
        * Affine::translate((-center.0, -center.1));

    // 参考矩形
    scene.set_paint(Color::from_rgba8(180, 180, 180, 120));
    scene.fill_rect(&Rect::new(300.0, 60.0, 400.0, 140.0));

    // 旋转后的矩形
    scene.set_transform(rotate_30);
    scene.set_paint(Color::from_rgb8(220, 100, 60));
    scene.fill_rect(&Rect::new(300.0, 60.0, 400.0, 140.0));
    scene.reset_transform();

    // ══════════════════════════════════════════════════════════════
    // 第 3 部分：缩放 (scale)
    // ══════════════════════════════════════════════════════════════

    // 参考圆
    scene.set_paint(Color::from_rgba8(180, 180, 180, 120));
    let circle_ref = kurbo::Circle::new((560.0, 100.0), 40.0).to_path(0.01);
    scene.fill_path(&circle_ref);

    // 缩放 1.8 倍，以 (560, 100) 为中心
    let scale_center = (560.0, 100.0);
    let scale_18 = Affine::translate(scale_center)
        * Affine::scale(1.8)
        * Affine::translate((-scale_center.0, -scale_center.1));

    scene.set_transform(scale_18);
    scene.set_paint(Color::from_rgb8(80, 180, 100));
    scene.fill_path(&circle_ref);
    scene.reset_transform();

    // ══════════════════════════════════════════════════════════════
    // 第 4 部分：变换组合 —— 旋转的矩形阵列
    // ══════════════════════════════════════════════════════════════

    // 以 (200, 350) 为中心，绘制 8 个旋转的矩形，形成花瓣图案
    let flower_center = (200.0, 350.0);
    let colors = [
        Color::from_rgb8(230, 70, 70),
        Color::from_rgb8(230, 150, 50),
        Color::from_rgb8(230, 210, 50),
        Color::from_rgb8(100, 200, 70),
        Color::from_rgb8(60, 180, 200),
        Color::from_rgb8(80, 100, 220),
        Color::from_rgb8(160, 70, 210),
        Color::from_rgb8(220, 80, 160),
    ];

    for i in 0..8 {
        let angle = (i as f64) * std::f64::consts::PI / 4.0;
        let transform = Affine::translate(flower_center)
            * Affine::rotate(angle)
            * Affine::translate((-flower_center.0, -flower_center.1));

        scene.set_transform(transform);
        scene.set_paint(colors[i]);
        // 矩形中心在 flower_center 上方 60px 处，旋转后形成花瓣
        scene.fill_rect(&Rect::new(
            flower_center.0 - 20.0,
            flower_center.1 - 80.0,
            flower_center.0 + 20.0,
            flower_center.1 - 20.0,
        ));
    }
    scene.reset_transform();

    // 花心
    scene.set_paint(Color::from_rgb8(255, 255, 255));
    let center_circle = kurbo::Circle::new(flower_center, 18.0).to_path(0.01);
    scene.fill_path(&center_circle);

    // ══════════════════════════════════════════════════════════════
    // 第 5 部分：错切 (skew) 与非均匀缩放
    // ══════════════════════════════════════════════════════════════

    // 错切变换：Affine::new([a, b, c, d, e, f]) 对应矩阵
    // | a c e |
    // | b d f |
    // | 0 0 1 |
    // 水平错切：c != 0；垂直错切：b != 0

    // 参考文字框
    scene.set_paint(Color::from_rgba8(180, 180, 180, 120));
    scene.fill_rect(&Rect::new(420.0, 280.0, 620.0, 380.0));

    // 水平错切 0.3
    let skew_x = Affine::new([1.0, 0.0, 0.3, 1.0, 0.0, 0.0]);
    scene.set_transform(skew_x * Affine::translate((420.0, 280.0)));
    scene.set_paint(Color::from_rgb8(200, 120, 60));
    scene.fill_rect(&Rect::new(0.0, 0.0, 200.0, 100.0));
    scene.reset_transform();

    // 非均匀缩放：x 方向 1.5，y 方向 0.6
    scene.set_paint(Color::from_rgba8(180, 180, 180, 120));
    scene.fill_rect(&Rect::new(420.0, 430.0, 620.0, 530.0));

    let non_uniform = Affine::translate((420.0, 430.0))
        * Affine::scale_non_uniform(1.5, 0.6)
        * Affine::translate((-420.0, -430.0));
    scene.set_transform(non_uniform);
    scene.set_paint(Color::from_rgb8(100, 160, 220));
    scene.fill_rect(&Rect::new(420.0, 430.0, 620.0, 530.0));
    scene.reset_transform();

    // ══════════════════════════════════════════════════════════════
    // 第 6 部分：变换累积 —— 嵌套变换效果
    // ══════════════════════════════════════════════════════════════

    // vello_hybrid 的 set_transform 是"设置"而非"累积"。
    // 每次调用 set_transform 会替换当前变换矩阵。
    // 要实现嵌套变换，需要手动组合矩阵。

    // 示例：先平移到 (650, 350)，再旋转 45 度，再缩放 0.8
    let nested = Affine::translate((650.0, 350.0))
        * Affine::rotate(std::f64::consts::PI / 4.0)
        * Affine::scale(0.8);

    scene.set_transform(nested);
    scene.set_paint(Color::from_rgb8(180, 60, 180));
    scene.fill_rect(&Rect::new(-50.0, -50.0, 50.0, 50.0)); // 以原点为中心的矩形

    // 在同一变换下再画一个描边矩形（偏移）
    scene.set_paint(Color::from_rgb8(100, 30, 100));
    scene.set_stroke(Stroke::new(3.0));
    scene.stroke_rect(&Rect::new(-70.0, -70.0, 70.0, 70.0));
    scene.reset_transform();

    // ── 渲染 ─────────────────────────────────────────────────────
    common::render_scene_to_png(&scene, WIDTH as u32, HEIGHT as u32, "02_transforms.png")?;

    Ok(())
}
