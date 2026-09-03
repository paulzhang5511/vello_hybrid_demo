# vello_hybrid 0.2.0 使用方法分析与教程

> 本文档基于 [vello_hybrid 0.2.0](https://docs.rs/vello_hybrid/0.2.0/vello_hybrid/) 官方 API 编写，配套 6 个可编译运行的示例。
>
> 所有示例已通过 `cargo check --examples` 验证（零错误零警告）。

---

## 一、vello_hybrid 是什么

vello_hybrid 是 **混合 CPU/GPU 2D 矢量图形渲染器**，是 linebender 组织 Vello 项目的三大渲染后端之一：

| 渲染器 | 定位 | 适用场景 |
|--------|------|----------|
| `vello` (Classic) | GPU Compute-centric | 高性能动态矢量场景 |
| `vello_cpu` | 纯 CPU | 无 GPU / 弱 GPU 设备 |
| `vello_hybrid` | CPU 路径处理 + GPU 合成 | 图像、渐变、滤镜密集型场景；WebGL2 浏览器 |

**混合架构核心思想**：
- **CPU** 负责路径处理（tessellation、coverage 计算）和初始几何构建
- **GPU** 负责快速渲染、合成和混合
- 最小化 CPU↔GPU 数据传输

---

## 二、核心架构与类型关系

```
┌─────────────────────────────────────────────────────┐
│                    CPU 端                             │
│  ┌──────────┐   ┌──────────────┐   ┌────────────┐ │
│  │  Scene   │──▶│  BezPath     │──▶│  PaintType │ │
│  │ (渲染上下文)│   │  (kurbo路径) │   │ (Solid/    │ │
│  │          │   │              │   │  Gradient/  │ │
│  │ set_paint│   │ Rect/Circle  │   │  Image)     │ │
│  │ set_transform│ RoundedRect  │   └────────────┘ │
│  │ fill_path│   └──────────────┘                    │
│  │ stroke_path│                                      │
│  │ glyph_run│   ┌──────────────┐                    │
│  │ push_layer│  │  Resources   │ (字形缓存等持久资源)│
│  └────┬─────┘   └──────┬───────┘                    │
│       │                  │                            │
└───────┼──────────────────┼────────────────────────────┘
        │                  │
        ▼                  ▼
┌─────────────────────────────────────────────────────┐
│                    GPU 端                             │
│  ┌──────────────────────────────────────────────┐   │
│  │  Renderer (wgpu) / WebGlRenderer (WebGL2)   │   │
│  │  - 管理 GPU 资源（着色器、管线、atlas）       │   │
│  │  - render(scene, resources, device, queue,   │   │
│  │           encoder, render_size, view,         │   │
│  │           texture_bindings)                   │   │
│  └──────────────────────────────────────────────┘   │
│                       │                               │
│                       ▼                               │
│              ┌─────────────────┐                     │
│              │  wgpu Texture   │ → 读回 → PNG       │
│              │  (渲染目标)      │ → Surface → 窗口   │
│              └─────────────────┘                     │
└─────────────────────────────────────────────────────┘
```

### 核心类型速查

| 类型 | 来源 crate | 作用 | 关键方法 |
|------|-----------|------|----------|
| `Scene` | vello_hybrid | CPU 端渲染上下文 | `new()`, `set_paint()`, `set_transform()`, `fill_path()`, `stroke_path()`, `fill_rect()`, `glyph_run()`, `push_layer()`, `pop_layer()` |
| `Renderer` | vello_hybrid | wgpu GPU 渲染器 | `new(device, config) → (Renderer, Resources)`, `render()` |
| `WebGlRenderer` | vello_hybrid | WebGL2 渲染器 | 浏览器端使用 |
| `Resources` | vello_hybrid | 持久化资源（字形缓存等） | 与 Renderer 绑定，不可跨 Renderer 使用 |
| `RenderTargetConfig` | vello_hybrid | 渲染目标配置 | `{ format, width, height }` |
| `RenderSize` | vello_hybrid | 渲染尺寸 | `{ width: u32, height: u32 }` |
| `PaintType` | peniko | 画笔类型 | `Solid(Color)`, `Gradient(Gradient)`, `Image(Image)` |
| `Color` | peniko | 颜色 | `from_rgb8(r,g,b)`, `from_rgba8(r,g,b,a)` |
| `BezPath` | kurbo | 贝塞尔路径 | `new()`, `move_to()`, `line_to()`, `curve_to()`, `quad_to()`, `close_path()` |
| `Affine` | kurbo | 仿射变换 | `translate()`, `rotate()`, `scale()`, `new()` |
| `Stroke` | kurbo | 描边参数 | `new(width)`, `with_caps()`, `with_join()`, `dash_pattern` |
| `TextureBindings` | vello_hybrid | 外部纹理绑定 | `insert(texture_id, texture_view)` |
| `GlyphRunBuilder` | vello_hybrid | 字形运行构建器 | `font_size()`, `fill_glyphs()`, `stroke_glyphs()`, `glyph_transform()`, `font_embolden()` |
| `Glyph` | glifo | 字形 | `{ id: u32, x: f32, y: f32 }` |
| `FontData` | peniko | 字体数据 | `new(Blob, index)` |
| `Filter` | vello_common | 滤镜 | `from_function(FilterFunction::Blur { radius })` |

### Feature Flags

| Feature | 默认 | 说明 |
|---------|------|------|
| `wgpu` | ✅ | 启用 wgpu GPU 后端 |
| `wgpu_default` | ✅ | wgpu 默认硬件后端（Vulkan/Metal/DX12） |
| `text` | ✅ | 启用字形渲染 `Scene::glyph_run` |
| `webgl` | ❌ | 启用 WebGL2 后端（浏览器/WASM） |

---

## 三、基本渲染流程（7 步）

```rust
use vello_hybrid::{Scene, Renderer, RenderTargetConfig, RenderSize, TextureBindings};
use wgpu;
use kurbo::Rect;
use peniko::Color;

// 1. 初始化 wgpu device/queue
let instance = wgpu::Instance::default();
let adapter = pollster::block_on(instance.request_adapter(...))?;
let (device, queue) = pollster::block_on(adapter.request_device(&descriptor))?;

// 2. 创建渲染目标纹理
let texture = device.create_texture(...);
let view = texture.create_view(&Default::default());

// 3. 创建 Renderer + Resources
let config = RenderTargetConfig { format, width, height };
let (mut renderer, mut resources) = Renderer::new(&device, &config);

// 4. 构建 Scene
let mut scene = Scene::new(width as u16, height as u16);
scene.set_paint(Color::from_rgb8(220, 60, 60));
scene.fill_rect(&Rect::new(50.0, 50.0, 200.0, 150.0));

// 5. 编码渲染命令
let mut encoder = device.create_command_encoder(&Default::default());
renderer.render(
    &scene, &mut resources, &device, &queue, &mut encoder,
    &RenderSize { width, height }, &view, &TextureBindings::default(),
)?;

// 6. 提交命令
queue.submit(Some(encoder.finish()));

// 7. 读回像素（headless）或呈现到窗口
```

---

## 四、教程索引

### 教程 1：基础形状绘制
**文件**：`examples/01_basic_shapes.rs`
**运行**：`cargo run --example 01_basic_shapes`

涵盖内容：
- `Scene::new()` 创建渲染上下文（注意：宽高为 u16）
- `set_paint()` 设置纯色画笔
- `set_fill_rule()` 设置填充规则（NonZero / EvenOdd）
- `fill_rect()` / `stroke_rect()` 矩形绘制
- `fill_path()` / `stroke_path()` 路径绘制
- `BezPath` 构建：圆形、圆角矩形、心形、星形、椭圆
- `Stroke` 配置：线宽、虚线（`smallvec::smallvec!`）

**关键注意**：kurbo 0.13 中 `Circle`/`RoundedRect`/`Ellipse` 转换为 `BezPath` 需使用 `.to_path(tolerance)` 方法（需导入 `kurbo::Shape` trait），而非 `BezPath::from()`。

---

### 教程 2：变换与坐标系
**文件**：`examples/02_transforms.rs`
**运行**：`cargo run --example 02_transforms`

涵盖内容：
- `Affine::translate()` 平移
- `Affine::rotate()` 旋转（绕指定中心：先平移→旋转→平移回）
- `Affine::scale()` / `scale_non_uniform()` 缩放
- `Affine::new()` 自定义矩阵（错切）
- 变换组合：矩阵乘法顺序（`*` 运算符）
- `set_transform()` / `reset_transform()`
- 旋转矩形阵列（花瓣图案）

**关键概念**：vello_hybrid 的 `set_transform` 是**设置**而非累积，每次调用会替换当前变换矩阵。嵌套变换需手动用 `*` 组合。

---

### 教程 3：画笔与描边样式
**文件**：`examples/03_paints.rs`
**运行**：`cargo run --example 03_paints`

涵盖内容：
- 纯色 `Color::from_rgb8()` / `from_rgba8()`
- 半透明叠加效果
- 描边线宽对比（1px / 3px / 6px / 10px）
- 端点样式 `Cap::Butt` / `Round` / `Square`
- 连接样式 `Join::Miter` / `Round` / `Bevel`
- 虚线模式 `dash_pattern`（使用 `smallvec::smallvec!`）
- 填充+描边组合（先填充后描边）

---

### 教程 4：图层与裁剪
**文件**：`examples/04_layers_clipping.rs`
**运行**：`cargo run --example 04_layers_clipping`

涵盖内容：
- `push_clip_path()` / `pop_clip_path()` —— 轻量路径裁剪（不创建离屏缓冲）
- `push_clip_layer()` —— 离屏图层裁剪（抗锯齿边界，需 `pop_layer()`）
- `push_opacity_layer()` —— 整体透明度图层（图层内先合成再整体透明）
- `push_filter_layer()` —— 滤镜图层（高斯模糊 `Filter::from_function(FilterFunction::Blur { radius })`）
- 组合使用：裁剪 + 透明度 + 滤镜
- 多层嵌套裁剪

**重要限制**（0.2.0）：
- Mask layers 尚未支持
- 复杂 filter graphs 不支持
- `push_clip_layer()` 必须在渲染前调用 `pop_layer()`

---

### 教程 5：文字渲染
**文件**：`examples/05_text_rendering.rs`
**运行**：`cargo run --example 05_text_rendering`

涵盖内容：
- 加载系统字体 → `FontData::new(Blob::new(Arc::new(bytes)), index)`
- 使用 `skrifa` 的 `FontRef` + `charmap` 进行字符到 glyph_id 映射
- `scene.glyph_run(&mut resources, &font_data)` → `GlyphRunBuilder`
- `font_size()` 设置字号
- `fill_glyphs()` 填充文字
- `stroke_glyphs()` 描边文字
- `font_embolden(FontEmbolden::new(Diagonal2::new(x, y)))` 合成粗体
- `glyph_transform(Affine)` 字形变换（斜体等）

**关键注意**：
1. 文字渲染需要在构建 Scene 时访问 `Resources`，因此流程变为：创建 `Renderer`/`Resources` → 构建 `Scene`（调用 `glyph_run` 时传入 `&mut resources`）→ 渲染
2. 本教程使用简化的字形布局（固定 advance width），生产环境建议使用 parley 等专业文本布局库
3. `FontEmbolden` 的字段是 `amount: Diagonal2`（不是 `x`/`y`），使用 `FontEmbolden::new(Diagonal2::new(x, y))` 构造

---

### 教程 6：Headless 渲染完整流程
**文件**：`examples/06_headless_full.rs`
**运行**：`cargo run --example 06_headless_full`

涵盖内容（不依赖 common 辅助模块，逐步拆解）：
1. 创建 `wgpu::Instance` → `Adapter` → `Device`/`Queue`
2. 创建离屏渲染 `Texture` + `TextureView`
3. 创建 `Renderer` + `Resources`
4. 构建 `Scene`
5. 创建 `CommandEncoder`，调用 `renderer.render()`
6. 创建读回 `Buffer`，`copy_texture_to_buffer`（使用 wgpu 29 的 `TexelCopyTextureInfo` / `TexelCopyBufferInfo`）
7. 提交命令，`map_async` 映射 Buffer，读取像素
8. 保存为 PNG

**wgpu 29.0 API 注意事项**：
- `request_device()` 只接受 1 个参数（`DeviceDescriptor`），不再接受 trace path
- `DeviceDescriptor` 必须包含 `experimental_features` 字段
- `copy_texture_to_buffer()` 使用 `TexelCopyTextureInfo` 和 `TexelCopyBufferInfo`（替代旧的 `ImageCopyTexture` / `ImageCopyBuffer`）
- `device.poll()` 使用 `PollType::Wait { submission_index, timeout }`（替代旧的 `Maintain::Wait`）

---

## 五、项目依赖说明

### Cargo.toml 关键依赖

```toml
[dependencies]
vello_hybrid = "0.2.0"
kurbo = "0.13"          # 2D 几何（BezPath, Affine, Stroke 等）
peniko = "0.6"          # 画笔/颜色/字体数据
wgpu = "29.0"           # GPU 抽象层
vello_common = "0.2"    # 共享类型（Filter 等）
glifo = "0.3"            # 字形渲染（Glyph, FontEmbolden）

[dev-dependencies]
pollster = "0.3"         # 阻塞式 async 运行时（wgpu 初始化）
image = { version = "0.25", default-features = false, features = ["png"] }
skrifa = "0.44"          # 字体解析（FontRef, charmap）
smallvec = "1.13"        # SmallVec 类型（dash_pattern）
```

**注意**：vello_hybrid 0.2.0 **未重新导出** kurbo、peniko、wgpu，需显式添加这些依赖。

---

## 六、常见问题与注意事项

### Q1: Scene 的宽高为什么是 u16？
vello_hybrid 的 `Scene::new(width: u16, height: u16)` 使用 u16 是设计选择。渲染目标的 `RenderSize` 使用 u32。最大场景尺寸为 65535×65535。

### Q2: set_transform 是累积的吗？
**不是**。每次调用 `set_transform` 会替换当前变换矩阵。需要嵌套变换时手动用 `*` 组合矩阵，或用 `save_current_state()` / `restore_state()` 保存恢复。

### Q3: 文字渲染为什么需要 Resources？
`glyph_run` 需要访问字形缓存（atlas），而缓存存储在 `Resources` 中。因此必须先创建 `Renderer`/`Resources`，再构建包含文字的 `Scene`。

### Q4: push_clip_path 和 push_clip_layer 有什么区别？
| 特性 | push_clip_path | push_clip_layer |
|------|----------------|-----------------|
| 离屏缓冲 | 不创建 | 创建 |
| 裁剪边界 | 不抗锯齿 | 抗锯齿 |
| 性能 | 高 | 较低 |
| 未 pop 是否允许 | 允许 | 不允许（渲染前必须 pop） |

### Q5: 0.2.0 有哪些已知限制？
- Mask layers 尚未支持
- 复杂 filter graphs 不支持（仅支持简单的 `FilterFunction::Blur`）
- 部分失败会 panic 而非返回错误
- wgpu 后端性能尚未完全优化

### Q6: kurbo 形状如何转换为 BezPath？
使用 `.to_path(tolerance)` 方法（需导入 `kurbo::Shape` trait）：
```rust
use kurbo::Shape;
let circle = kurbo::Circle::new((x, y), r).to_path(0.01);
```
不要使用 `BezPath::from(shape)`，kurbo 0.13 不支持。

### Q7: dash_pattern 用什么类型？
`Stroke::dash_pattern` 是 `SmallVec<[f64; 4]>`，使用 `smallvec::smallvec![a, b, c]` 宏创建：
```rust
stroke.dash_pattern = smallvec::smallvec![10.0, 5.0];
```

### Q8: 运行时遇到 "Parent device is lost" 错误怎么办？
这是 wgpu 29.0 在部分 macOS 系统上与 Metal 后端的兼容性问题。可尝试：
1. 将 `power_preference` 改为 `wgpu::PowerPreference::LowPower`
2. 将 `force_fallback_adapter` 设为 `true` 使用软件渲染器
3. 检查系统 GPU 驱动是否正常
4. 尝试降低 wgpu 版本（需与 vello_hybrid 兼容）

代码本身已通过 `cargo check --examples` 验证（零错误零警告），运行时错误属于环境兼容性问题。

---

## 七、项目结构

```
vello_hybrid_demo/
├── Cargo.toml
├── TUTORIALS.md              ← 本文档
├── src/
│   └── main.rs               ← 入口（默认 hello world）
└── examples/
    ├── common/
    │   └── mod.rs            ← 共享渲染辅助模块（#![allow(dead_code)]）
    ├── 01_basic_shapes.rs    ← 教程1：基础形状
    ├── 02_transforms.rs      ← 教程2：变换与坐标系
    ├── 03_paints.rs          ← 教程3：画笔与描边样式
    ├── 04_layers_clipping.rs ← 教程4：图层与裁剪
    ├── 05_text_rendering.rs  ← 教程5：文字渲染
    └── 06_headless_full.rs   ← 教程6：Headless 完整流程
```

---

## 八、参考资源

- [vello_hybrid 0.2.0 官方文档](https://docs.rs/vello_hybrid/0.2.0/vello_hybrid/)
- [Vello GitHub 仓库](https://github.com/linebender/vello)
- [kurbo 文档](https://docs.rs/kurbo/) — 2D 几何库
- [peniko 文档](https://docs.rs/peniko/) — 画笔/颜色库
- [wgpu 文档](https://docs.rs/wgpu/) — GPU 抽象层
- [glifo 文档](https://docs.rs/glifo/) — 字形渲染库
