# vello_hybrid_demo

[![vello_hybrid](https://img.shields.io/badge/vello__hybrid-0.2.0-orange)](https://docs.rs/vello_hybrid/0.2.0/vello_hybrid/)
[![Rust](https://img.shields.io/badge/rust-2024-blue)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

`vello_hybrid` 0.2.0 的使用方法分析与 6 个可运行教程示例。

> **vello_hybrid** 是 linebender Vello 项目的混合 CPU/GPU 2D 矢量图形渲染器——CPU 负责路径处理，GPU 负责快速渲染与合成。

---

## 特性

- ✅ **6 个完整教程**，覆盖从基础形状到 Headless 渲染全流程
- ✅ **零编译错误零警告**，全部通过 `cargo check --examples` 验证
- ✅ **共享渲染辅助模块**，封装 wgpu headless 渲染流程
- ✅ **详细 API 分析文档**，含类型速查表、常见问题解答
- ✅ **wgpu 29.0 兼容**，适配最新 wgpu API（TexelCopyTextureInfo 等）

---

## 快速开始

### 系统化教程

📖 **[GUIDE.md — vello_hybrid 从新手到高手完全指南](GUIDE.md)**

包含 15 个学习等级、15+ 张流程图，覆盖从概念架构到工程化最佳实践的完整路径：
- 🟢 新手入门（L1-L3）：概念、环境、Hello World
- 🟡 进阶掌握（L4-L7）：核心概念、基础绘制、变换、画笔
- 🟠 高级应用（L8-L11）：图层裁剪、文字渲染、Headless、性能优化
- 🔴 专家精通（L12-L15）：架构原理、高级技巧、调试排错、最佳实践

### 前置要求

- Rust 工具链（edition 2024）
- 支持 wgpu 的 GPU（Vulkan / Metal / DX12），或使用软件渲染 fallback

### 克隆与运行

```bash
git clone https://github.com/paulzhang5511/vello_hybrid_demo.git
cd vello_hybrid_demo

# 编译检查所有示例
cargo check --examples

# 运行教程1（输出 01_basic_shapes.png）
cargo run --example 01_basic_shapes

# 运行教程6（完整 Headless 渲染流程）
cargo run --example 06_headless_full
```

---

## 教程列表

| # | 教程 | 文件 | 核心内容 |
|---|------|------|----------|
| 1 | 基础形状绘制 | `examples/01_basic_shapes.rs` | Scene 构建、fill/stroke、BezPath、心形/星形/椭圆 |
| 2 | 变换与坐标系 | `examples/02_transforms.rs` | Affine 平移/旋转/缩放/错切、矩阵组合、花瓣阵列 |
| 3 | 画笔与描边样式 | `examples/03_paints.rs` | 纯色/半透明、线宽、Cap/Join、虚线、填充+描边组合 |
| 4 | 图层与裁剪 | `examples/04_layers_clipping.rs` | clip_path/clip_layer、opacity_layer、filter_layer（高斯模糊）、嵌套裁剪 |
| 5 | 文字渲染 | `examples/05_text_rendering.rs` | glyph_run、font_size、fill/stroke_glyphs、font_embolden、glyph_transform |
| 6 | Headless 完整流程 | `examples/06_headless_full.rs` | wgpu Instance→Adapter→Device→Texture→Renderer→读回像素→PNG |

每个示例运行后会在项目根目录生成对应的 PNG 图片。

---

## 项目结构

```
vello_hybrid_demo/
├── Cargo.toml              # 项目依赖配置
├── README.md               # 本文件
├── TUTORIALS.md            # 详细教程文档（API 分析 + 常见问题）
├── src/
│   └── main.rs             # 默认入口
└── examples/
    ├── common/
    │   └── mod.rs          # 共享渲染辅助模块（#![allow(dead_code)]）
    ├── 01_basic_shapes.rs
    ├── 02_transforms.rs
    ├── 03_paints.rs
    ├── 04_layers_clipping.rs
    ├── 05_text_rendering.rs
    └── 06_headless_full.rs
```

---

## 核心依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| `vello_hybrid` | 0.2.0 | 混合 CPU/GPU 2D 渲染器 |
| `wgpu` | 29.0 | GPU 抽象层 |
| `kurbo` | 0.13 | 2D 几何（BezPath, Affine, Stroke） |
| `peniko` | 0.6 | 画笔/颜色/字体数据 |
| `vello_common` | 0.2 | 共享类型（Filter 等） |
| `glifo` | 0.3 | 字形渲染（Glyph, FontEmbolden） |
| `skrifa` | 0.44 | 字体解析（FontRef, charmap） |
| `pollster` | 0.3 | 阻塞式 async 运行时 |
| `image` | 0.25 | PNG 编码 |
| `smallvec` | 1.13 | SmallVec（dash_pattern） |

> **注意**：vello_hybrid 0.2.0 未重新导出 kurbo / peniko / wgpu，需显式添加依赖。

---

## 核心架构

```
┌─────────────────────────────────────────────────────┐
│                    CPU 端                             │
│  ┌──────────┐   ┌──────────────┐   ┌────────────┐ │
│  │  Scene   │──▶│  BezPath     │──▶│  PaintType │ │
│  │ set_paint│   │  (kurbo)     │   │ (peniko)   │ │
│  │ fill_path│   └──────────────┘   └────────────┘ │
│  │ glyph_run│   ┌──────────────┐                    │
│  │ push_layer│  │  Resources   │ (字形缓存)        │
│  └────┬─────┘   └──────┬───────┘                    │
└───────┼──────────────────┼────────────────────────────┘
        │                  │
        ▼                  ▼
┌─────────────────────────────────────────────────────┐
│                    GPU 端                             │
│  ┌──────────────────────────────────────────────┐   │
│  │  Renderer (wgpu)                              │   │
│  │  render(scene, resources, device, queue, ...)│   │
│  └──────────────────────────────────────────────┘   │
│                       │                               │
│                       ▼                               │
│              ┌─────────────────┐                     │
│              │  wgpu Texture   │ → 读回 → PNG       │
│              └─────────────────┘                     │
└─────────────────────────────────────────────────────┘
```

---

## 常见问题

### Q: 运行时遇到 "Parent device is lost" 错误？

这是 wgpu 29.0 在部分 macOS 系统上与 Metal 后端的兼容性问题。可尝试：
- 将 `power_preference` 改为 `wgpu::PowerPreference::LowPower`
- 将 `force_fallback_adapter` 设为 `true` 使用软件渲染器
- 检查系统 GPU 驱动是否正常

### Q: kurbo 形状如何转换为 BezPath？

使用 `.to_path(tolerance)` 方法（需导入 `kurbo::Shape` trait）：
```rust
use kurbo::Shape;
let circle = kurbo::Circle::new((x, y), r).to_path(0.01);
```

### Q: dash_pattern 用什么类型？

`SmallVec<[f64; 4]>`，使用 `smallvec::smallvec!` 宏：
```rust
stroke.dash_pattern = smallvec::smallvec![10.0, 5.0];
```

更多常见问题见 [TUTORIALS.md](TUTORIALS.md)。

---

## 参考资源

- [vello_hybrid 0.2.0 官方文档](https://docs.rs/vello_hybrid/0.2.0/vello_hybrid/)
- [Vello GitHub 仓库](https://github.com/linebender/vello)
- [kurbo 文档](https://docs.rs/kurbo/)
- [peniko 文档](https://docs.rs/peniko/)
- [wgpu 文档](https://docs.rs/wgpu/)

---

## 许可证

MIT License
