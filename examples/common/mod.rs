//! 共享渲染辅助模块
//!
//! 封装 wgpu headless 渲染流程，让各教程示例专注于 Scene 构建。
//!
//! 核心函数：[`render_scene_to_png`] —— 将 vello_hybrid::Scene 渲染为 PNG 文件。

#![allow(dead_code)]

use vello_hybrid::{
    Renderer,
    RenderSize,
    RenderTargetConfig,
    Scene,
    TextureBindings,
};
use kurbo::Shape;
use wgpu::{
    self,
    Device,
    Extent3d,
    Origin3d,
    PollType,
    Queue,
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

/// 渲染场景并保存为 PNG 文件。
///
/// # 参数
/// - `scene`: 已构建好的 vello_hybrid 场景
/// - `width`: 输出图像宽度（像素）
/// - `height`: 输出图像高度（像素）
/// - `output_path`: PNG 文件输出路径
///
/// # 示例
/// ```no_run
/// let mut scene = vello_hybrid::Scene::new(800, 600);
/// // ... 构建场景 ...
/// common::render_scene_to_png(&scene, 800, 600, "output.png").unwrap();
/// ```
pub fn render_scene_to_png(
    scene: &Scene,
    width: u32,
    height: u32,
    output_path: &str,
) -> Result<(), String> {
    // 1. 初始化 wgpu（阻塞式，适用于 headless 环境）
    let instance = wgpu::Instance::default();

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .map_err(|e| format!("请求适配器失败: {e}"))?;

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("vello_hybrid_device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::default(),
        },
    ))
    .map_err(|e| format!("请求设备失败: {e}"))?;

    // 2. 创建离屏渲染纹理
    let texture_format = TextureFormat::Rgba8Unorm;
    let render_texture = device.create_texture(&TextureDescriptor {
        label: Some("vello_hybrid_render_target"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: texture_format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let texture_view = render_texture.create_view(&TextureViewDescriptor::default());

    // 3. 创建 vello_hybrid Renderer + Resources
    let render_target_config = RenderTargetConfig {
        format: texture_format,
        width,
        height,
    };
    let (mut renderer, mut resources) = Renderer::new(&device, &render_target_config);

    // 4. 创建 CommandEncoder 并执行渲染
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("vello_hybrid_encoder"),
    });

    let render_size = RenderSize { width, height };
    let texture_bindings = TextureBindings::default();

    renderer
        .render(
            scene,
            &mut resources,
            &device,
            &queue,
            &mut encoder,
            &render_size,
            &texture_view,
            &texture_bindings,
        )
        .map_err(|e| format!("渲染失败: {e:?}"))?;

    // 5. 创建读回 Buffer，将纹理数据拷贝到 CPU 可访问的 buffer
    let bytes_per_row = width * 4; // Rgba8 = 4 bytes per pixel
    let buffer_size = (bytes_per_row * height) as u64;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vello_hybrid_readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture: &render_texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    // 6. 提交命令并等待 GPU 完成
    queue.submit(Some(encoder.finish()));

    // 7. 映射 buffer 并读取像素数据
    let buffer_slice = output_buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });
    let _ = device.poll(PollType::Wait { submission_index: None, timeout: None });
    receiver
        .recv()
        .map_err(|e| format!("接收映射结果失败: {e}"))?
        .map_err(|e| format!("映射 buffer 失败: {e}"))?;

    let data = buffer_slice.get_mapped_range();
    let pixel_data: Vec<u8> = data.to_vec();
    drop(data);
    output_buffer.unmap();

    // 8. 保存为 PNG（vello_hybrid 输出预乘 alpha，image crate 直接保存）
    let img = image::RgbaImage::from_raw(width, height, pixel_data)
        .ok_or_else(|| "构建图像失败".to_string())?;
    img.save(output_path)
        .map_err(|e| format!("保存 PNG 失败: {e}"))?;

    println!("已保存渲染结果: {output_path} ({width}x{height})");
    Ok(())
}

/// 构建一个圆形 BezPath 的便捷函数。
///
/// vello_hybrid 使用 kurbo::BezPath 描述矢量路径。
pub fn circle(cx: f64, cy: f64, r: f64) -> kurbo::BezPath {
    kurbo::Circle::new((cx, cy), r).to_path(0.01)
}

/// 构建一个圆角矩形 BezPath 的便捷函数。
pub fn rounded_rect(x: f64, y: f64, w: f64, h: f64, radius: f64) -> kurbo::BezPath {
    kurbo::RoundedRect::new(x, y, x + w, y + h, radius).to_path(0.01)
}

// ══════════════════════════════════════════════════════════════════
// 文字渲染支持：需要在构建 Scene 时访问 Resources
// ══════════════════════════════════════════════════════════════════

use vello_hybrid::Resources;

/// 初始化 wgpu 设备和队列（阻塞式）。
pub fn init_wgpu() -> Result<(Device, Queue), String> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .map_err(|e| format!("请求适配器失败: {e}"))?;

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("vello_hybrid_device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::default(),
        },
    ))
    .map_err(|e| format!("请求设备失败: {e}"))?;

    Ok((device, queue))
}

/// 创建 vello_hybrid Renderer 和关联的 Resources。
///
/// 文字渲染（glyph_run）需要在构建 Scene 时访问 Resources，
/// 因此需要先创建 Renderer/Resources，再构建 Scene。
pub fn create_renderer(
    device: &Device,
    width: u32,
    height: u32,
) -> (Renderer, Resources) {
    let render_target_config = RenderTargetConfig {
        format: TextureFormat::Rgba8Unorm,
        width,
        height,
    };
    Renderer::new(device, &render_target_config)
}

/// 使用已有的 Renderer/Resources 渲染场景并保存为 PNG。
///
/// 适用于文字渲染等需要在场景构建阶段访问 Resources 的场景。
pub fn render_with_renderer_to_png(
    device: &Device,
    queue: &Queue,
    renderer: &mut Renderer,
    resources: &mut Resources,
    scene: &Scene,
    width: u32,
    height: u32,
    output_path: &str,
) -> Result<(), String> {
    let texture_format = TextureFormat::Rgba8Unorm;
    let render_texture = device.create_texture(&TextureDescriptor {
        label: Some("vello_hybrid_render_target"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: texture_format,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let texture_view = render_texture.create_view(&TextureViewDescriptor::default());

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("vello_hybrid_encoder"),
    });

    let render_size = RenderSize { width, height };
    let texture_bindings = TextureBindings::default();

    renderer
        .render(
            scene,
            resources,
            device,
            queue,
            &mut encoder,
            &render_size,
            &texture_view,
            &texture_bindings,
        )
        .map_err(|e| format!("渲染失败: {e:?}"))?;

    let bytes_per_row = width * 4;
    let buffer_size = (bytes_per_row * height) as u64;
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vello_hybrid_readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture: &render_texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(Some(encoder.finish()));

    let buffer_slice = output_buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });
    let _ = device.poll(PollType::Wait { submission_index: None, timeout: None });
    receiver
        .recv()
        .map_err(|e| format!("接收映射结果失败: {e}"))?
        .map_err(|e| format!("映射 buffer 失败: {e}"))?;

    let data = buffer_slice.get_mapped_range();
    let pixel_data: Vec<u8> = data.to_vec();
    drop(data);
    output_buffer.unmap();

    let img = image::RgbaImage::from_raw(width, height, pixel_data)
        .ok_or_else(|| "构建图像失败".to_string())?;
    img.save(output_path)
        .map_err(|e| format!("保存 PNG 失败: {e}"))?;

    println!("已保存渲染结果: {output_path} ({width}x{height})");
    Ok(())
}
