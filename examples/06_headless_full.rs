//! 教程 06：Headless 渲染完整流程
//!
//! 本教程展示 vello_hybrid 的完整 headless 渲染流程，
//! 不依赖 common 辅助模块，逐步拆解每一步：
//!
//! 1. 创建 wgpu Instance → Adapter → Device/Queue
//! 2. 创建离屏渲染纹理 (Texture + TextureView)
//! 3. 创建 vello_hybrid::Renderer + Resources
//! 4. 构建 Scene（设置画笔、绘制形状）
//! 5. 创建 CommandEncoder，调用 renderer.render()
//! 6. 创建读回 Buffer，copy_texture_to_buffer
//! 7. 提交命令，映射 Buffer，读取像素
//! 8. 保存为 PNG
//!
//! 运行：cargo run --example 06_headless_full
//! 输出：06_headless_full.png

use vello_hybrid::{
    Renderer,
    RenderSize,
    RenderTargetConfig,
    Scene,
    TextureBindings,
};
use kurbo::{Affine, Rect, Shape, Stroke};
use peniko::Color;
use wgpu::{
    self,
    Extent3d,
    Origin3d,
    PollType,
    TexelCopyBufferInfo,
    TexelCopyBufferLayout,
    TexelCopyTextureInfo,
    TextureAspect,
    TextureDescriptor,
    TextureDimension,
    TextureFormat,
    TextureUsages,
    TextureViewDescriptor,
};

fn main() -> Result<(), String> {
    const WIDTH: u32 = 600;
    const HEIGHT: u32 = 400;

    // ══════════════════════════════════════════════════════════════
    // 第 1 步：创建 wgpu Instance → Adapter → Device/Queue
    // ══════════════════════════════════════════════════════════════
    //
    // Instance 是 wgpu 的入口，负责枚举 GPU 适配器。
    // Adapter 代表一个物理 GPU（或软件渲染器）。
    // Device 是逻辑设备，用于创建 GPU 资源。
    // Queue 是命令队列，用于提交渲染命令。

    println!("1. 创建 wgpu Instance...");
    let instance = wgpu::Instance::default();

    println!("2. 请求 Adapter...");
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        // 优先使用高性能 GPU（独立显卡）
        power_preference: wgpu::PowerPreference::HighPerformance,
        // 离屏渲染不需要 surface
        compatible_surface: None,
        // 不强制使用 fallback（软件渲染）
        force_fallback_adapter: false,
    }))
    .map_err(|e| format!("请求适配器失败: {e}"))?;

    println!("3. 请求 Device/Queue...");
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("vello_hybrid_tutorial_device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::default(),
        },
    ))
    .map_err(|e| format!("请求设备失败: {e}"))?;

    // ══════════════════════════════════════════════════════════════
    // 第 2 步：创建离屏渲染纹理
    // ══════════════════════════════════════════════════════════════
    //
    // vello_hybrid 渲染到 wgpu::TextureView。
    // 我们创建一个 2D 纹理作为渲染目标，同时允许从它拷贝数据（COPY_SRC）。

    println!("4. 创建离屏渲染纹理...");
    let texture_format = TextureFormat::Rgba8Unorm;

    let render_texture = device.create_texture(&TextureDescriptor {
        label: Some("render_target"),
        size: Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: texture_format,
        // RENDER_ATTACHMENT: 作为渲染目标
        // COPY_SRC: 允许拷贝到 buffer（用于读回像素）
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    // TextureView 是 Texture 的一个"视图"，渲染器实际操作的是 View
    let texture_view = render_texture.create_view(&TextureViewDescriptor::default());

    // ══════════════════════════════════════════════════════════════
    // 第 3 步：创建 vello_hybrid Renderer + Resources
    // ══════════════════════════════════════════════════════════════
    //
    // Renderer 管理 GPU 资源（着色器、管线、atlas 等）。
    // Resources 是持久化资源，与 Renderer 绑定使用。
    // 注意：一个 Resources 只能与创建它的 Renderer 一起使用。

    println!("5. 创建 vello_hybrid Renderer...");
    let render_target_config = RenderTargetConfig {
        format: texture_format,
        width: WIDTH,
        height: HEIGHT,
    };

    let (mut renderer, mut resources) = Renderer::new(&device, &render_target_config);

    // ══════════════════════════════════════════════════════════════
    // 第 4 步：构建 Scene
    // ══════════════════════════════════════════════════════════════
    //
    // Scene 是 CPU 端的渲染上下文，负责路径处理和几何构建。
    // 这一步完全在 CPU 上执行，不涉及 GPU。

    println!("6. 构建 Scene...");
    let mut scene = Scene::new(WIDTH as u16, HEIGHT as u16);

    // 背景渐变（用纯色模拟）
    scene.set_paint(Color::from_rgb8(30, 30, 50));
    scene.fill_rect(&Rect::new(0.0, 0.0, WIDTH as f64, HEIGHT as f64));

    // 绘制一个发光的圆形（用多层半透明模拟）
    let center = (300.0, 200.0);
    for i in (0..5).rev() {
        let radius = 120.0 - (i as f64) * 20.0;
        let alpha = 30 + i * 40;
        scene.set_paint(Color::from_rgba8(100, 180, 255, alpha as u8));
        let circle = kurbo::Circle::new(center, radius).to_path(0.01);
        scene.fill_path(&circle);
    }

    // 中心实心圆
    scene.set_paint(Color::from_rgb8(220, 240, 255));
    let core = kurbo::Circle::new(center, 30.0).to_path(0.01);
    scene.fill_path(&core);

    // 周围的轨道矩形（旋转）
    for i in 0..8 {
        let angle = (i as f64) * std::f64::consts::PI / 4.0;
        let transform = Affine::translate(center)
            * Affine::rotate(angle)
            * Affine::translate((-center.0, -center.1));

        scene.set_transform(transform);
        scene.set_paint(Color::from_rgba8(255, 200, 100, 180));
        scene.fill_rect(&Rect::new(
            center.0 - 8.0,
            center.1 - 140.0,
            center.0 + 8.0,
            center.1 - 100.0,
        ));
    }
    scene.reset_transform();

    // 底部文字框
    scene.set_paint(Color::from_rgba8(255, 255, 255, 30));
    scene.fill_rect(&Rect::new(50.0, 320.0, 550.0, 370.0));

    scene.set_paint(Color::from_rgb8(200, 220, 255));
    scene.set_stroke(Stroke::new(2.0));
    scene.stroke_rect(&Rect::new(50.0, 320.0, 550.0, 370.0));

    // ══════════════════════════════════════════════════════════════
    // 第 5 步：创建 CommandEncoder，执行渲染
    // ══════════════════════════════════════════════════════════════
    //
    // CommandEncoder 用于录制 GPU 命令。
    // renderer.render() 将 Scene 编码为 GPU 绘制命令。

    println!("7. 编码渲染命令...");
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("main_encoder"),
    });

    let render_size = RenderSize {
        width: WIDTH,
        height: HEIGHT,
    };

    // 空的纹理绑定（本示例不使用外部纹理）
    let texture_bindings = TextureBindings::default();

    renderer
        .render(
            &scene,
            &mut resources,
            &device,
            &queue,
            &mut encoder,
            &render_size,
            &texture_view,
            &texture_bindings,
        )
        .map_err(|e| format!("渲染失败: {e:?}"))?;

    // ══════════════════════════════════════════════════════════════
    // 第 6 步：创建读回 Buffer，拷贝纹理数据
    // ══════════════════════════════════════════════════════════════
    //
    // GPU 纹理数据不能直接从 CPU 读取，需要先拷贝到一个可映射的 Buffer。

    println!("8. 创建读回 Buffer 并拷贝纹理...");
    let bytes_per_row = WIDTH * 4; // Rgba8 = 4 bytes per pixel
    let buffer_size = (bytes_per_row * HEIGHT) as u64;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback_buffer"),
        size: buffer_size,
        // MAP_READ: 允许 CPU 读取
        // COPY_DST: 允许作为拷贝目标
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        // 源：纹理
        TexelCopyTextureInfo {
            texture: &render_texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        // 目标：Buffer
        TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        // 拷贝区域大小
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );

    // ══════════════════════════════════════════════════════════════
    // 第 7 步：提交命令，等待 GPU 完成，映射 Buffer 读取像素
    // ══════════════════════════════════════════════════════════════

    println!("9. 提交命令并等待 GPU...");
    queue.submit(Some(encoder.finish()));

    // 映射 Buffer 为只读
    let buffer_slice = output_buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });

    // 阻塞等待 GPU 完成所有操作
    let _ = device.poll(PollType::Wait { submission_index: None, timeout: None });

    // 接收映射结果
    receiver
        .recv()
        .map_err(|e| format!("接收映射结果失败: {e}"))?
        .map_err(|e| format!("映射 buffer 失败: {e}"))?;

    // 读取像素数据
    println!("10. 读取像素数据...");
    let data = buffer_slice.get_mapped_range();
    let pixel_data: Vec<u8> = data.to_vec();
    drop(data); // 必须先 drop 才能 unmap
    output_buffer.unmap();

    // ══════════════════════════════════════════════════════════════
    // 第 8 步：保存为 PNG
    // ══════════════════════════════════════════════════════════════

    println!("11. 保存为 PNG...");
    let img = image::RgbaImage::from_raw(WIDTH, HEIGHT, pixel_data)
        .ok_or_else(|| "构建图像失败".to_string())?;
    img.save("06_headless_full.png")
        .map_err(|e| format!("保存 PNG 失败: {e}"))?;

    println!("完成！输出: 06_headless_full.png ({WIDTH}x{HEIGHT})");
    Ok(())
}
