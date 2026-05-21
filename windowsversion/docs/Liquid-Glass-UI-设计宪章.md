# Liquid Glass UI 设计宪章

> NCdesktop 视觉设计系统 — 基于 Apple Liquid Glass 设计语言
> 版本：1.0 | 生效日期：2026-03-25

---

## 第一章 · 设计哲学

### 1.1 Apple Liquid Glass 的本质

Liquid Glass 是 Apple 在 WWDC 2025 发布的全新设计语言，是自 iOS 7 以来最深远的视觉革新。它不是简单的"毛玻璃"效果，而是将**真实玻璃的光学属性**引入数字界面的材质系统——半透明、折射、反射、流体感，所有这些在实时响应用户交互和环境光照。

### 1.2 三大核心原则

```
┌─────────────────────────────────────────────────┐
│                                                 │
│   🔍 CLARITY（清晰）                            │
│   一眼就能理解。界面元素在任何背景上都保持       │
│   清晰可读，玻璃材质服务于可读性而非遮蔽。       │
│                                                 │
│   🎯 DEFERENCE（内容优先）                      │
│   内容永远是焦点。Liquid Glass 让导航和控件       │
│   在视觉上"退让"，让底层内容成为主角。           │
│                                                 │
│   📐 DEPTH（层次深度）                          │
│   通过视觉层次和真实感的光影创造空间感，          │
│   而非扁平化的堆叠。                             │
│                                                 │
└─────────────────────────────────────────────────┘
```

### 1.3 NCdesktop 的设计立场

本应用作为 macOS 原生桌面应用，应当**与系统视觉语言深度融合**，而非创造独立的视觉体系。设计的目标是让用户感觉这是"系统的一部分"，而非"运行在系统上的外来物"。

---

## 第二章 · 材质系统

### 2.1 Liquid Glass 材质光学原理

传统毛玻璃**散射**光线，而 Liquid Glass **折射**光线——像真正的玻璃一样弯曲、塑形和聚焦光线，在实时中提供分离感和层次感。

```
传统毛玻璃:     内容 → [散射/模糊] → 均匀的朦胧
Liquid Glass:   内容 → [折射/弯曲] → 动态的透视变形 + 高光
```

### 2.2 材质层级体系

在 Web 端（Tauri WebView）中，我们通过 CSS 模拟 Liquid Glass 的材质层级：

| 层级 | 名称 | 用途 | 模糊值 | 背景透明度 | 边框 |
|------|------|------|--------|------------|------|
| L1 | Ultra Thin | 大面积背景、侧边栏底层 | 8px | 0.03-0.06 | 无 |
| L2 | Thin | 面板背景、列表容器 | 12px | 0.06-0.10 | 1px, 0.08 alpha |
| L3 | Regular | 卡片、工具栏、标准玻璃面 | 16px | 0.10-0.18 | 1px, 0.12 alpha |
| L4 | Thick | 悬浮面板、弹出菜单、模态 | 24px | 0.14-0.22 | 1px, 0.15 alpha |
| L5 | Ultra Thick | 重要提示、聚焦对话框 | 32px | 0.20-0.30 | 1px, 0.18 alpha |

### 2.3 CSS 设计令牌（Design Tokens）

```css
:root {
  /* ═══ 材质令牌 ═══ */
  
  /* 模糊值 */
  --glass-blur-xs: 4px;
  --glass-blur-sm: 8px;
  --glass-blur-md: 12px;
  --glass-blur-lg: 16px;
  --glass-blur-xl: 24px;
  --glass-blur-2xl: 32px;

  /* 背景色 — 亮色模式 */
  --glass-bg-ultra-thin: rgba(255, 255, 255, 0.03);
  --glass-bg-thin: rgba(255, 255, 255, 0.06);
  --glass-bg-regular: rgba(255, 255, 255, 0.12);
  --glass-bg-thick: rgba(255, 255, 255, 0.18);
  --glass-bg-ultra-thick: rgba(255, 255, 255, 0.25);

  /* 边框 — 亮色模式 */
  --glass-border-subtle: rgba(255, 255, 255, 0.08);
  --glass-border-regular: rgba(255, 255, 255, 0.12);
  --glass-border-strong: rgba(255, 255, 255, 0.18);
  --glass-border-accent-top: rgba(255, 255, 255, 0.25);

  /* 阴影 */
  --glass-shadow-sm: 0 1px 4px rgba(0, 0, 0, 0.06);
  --glass-shadow-md: 0 4px 16px rgba(0, 0, 0, 0.10);
  --glass-shadow-lg: 0 8px 32px rgba(0, 0, 0, 0.15);
  --glass-shadow-xl: 0 16px 48px rgba(0, 0, 0, 0.20);
  --glass-shadow-inset: inset 0 1px 0 rgba(255, 255, 255, 0.10);

  /* ═══ 圆角令牌 ═══ */
  --radius-xs: 4px;
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
  --radius-xl: 20px;
  --radius-2xl: 24px;
  --radius-full: 9999px;

  /* ═══ 间距令牌（4px 基准网格）═══ */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
  --space-8: 32px;
  --space-10: 40px;
  --space-12: 48px;
  --space-16: 64px;

  /* ═══ 品牌色 ═══ */
  --brand-navy: #1F456E;
  --brand-navy-light: #2A5A8F;
  --brand-navy-dark: #152F4D;
  --brand-gold: #FFC000;
  --brand-gold-light: #FFD54F;
  --brand-gold-dark: #E5AC00;

  /* ═══ 语义色 ═══ */
  --color-primary: var(--brand-navy);
  --color-accent: var(--brand-gold);
  --color-success: #34C759;
  --color-warning: #FF9500;
  --color-danger: #FF3B30;
  --color-info: #5AC8FA;

  /* ═══ 文本色 ═══ */
  --text-primary: rgba(0, 0, 0, 0.85);
  --text-secondary: rgba(0, 0, 0, 0.55);
  --text-tertiary: rgba(0, 0, 0, 0.35);
  --text-on-glass: rgba(0, 0, 0, 0.80);
  --text-on-glass-secondary: rgba(0, 0, 0, 0.50);

  /* ═══ 排版令牌 ═══ */
  --font-system: -apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Pro Display",
    "Helvetica Neue", "PingFang SC", "Microsoft YaHei", sans-serif;
  --font-mono: "SF Mono", "Fira Code", "JetBrains Mono", 
    "Cascadia Code", Menlo, monospace;

  --text-xs: 11px;
  --text-sm: 13px;
  --text-base: 15px;
  --text-lg: 17px;
  --text-xl: 20px;
  --text-2xl: 24px;
  --text-3xl: 28px;
  --text-4xl: 34px;

  --leading-tight: 1.2;
  --leading-normal: 1.5;
  --leading-relaxed: 1.65;

  --tracking-tight: -0.02em;
  --tracking-normal: 0;
  --tracking-wide: 0.02em;

  /* ═══ 动画令牌 ═══ */
  --duration-instant: 100ms;
  --duration-fast: 200ms;
  --duration-normal: 300ms;
  --duration-slow: 500ms;
  --duration-glacial: 800ms;
  --ease-out-expo: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-out-quart: cubic-bezier(0.25, 1, 0.5, 1);
  --ease-in-out-quart: cubic-bezier(0.76, 0, 0.24, 1);
  --ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);
}

/* ═══ 暗色模式令牌 ═══ */
[data-theme="dark"],
@media (prefers-color-scheme: dark) {
  :root {
    --glass-bg-ultra-thin: rgba(30, 30, 30, 0.05);
    --glass-bg-thin: rgba(30, 30, 30, 0.10);
    --glass-bg-regular: rgba(30, 30, 30, 0.18);
    --glass-bg-thick: rgba(30, 30, 30, 0.25);
    --glass-bg-ultra-thick: rgba(30, 30, 30, 0.35);

    --glass-border-subtle: rgba(255, 255, 255, 0.06);
    --glass-border-regular: rgba(255, 255, 255, 0.10);
    --glass-border-strong: rgba(255, 255, 255, 0.15);
    --glass-border-accent-top: rgba(255, 255, 255, 0.20);

    --glass-shadow-sm: 0 1px 4px rgba(0, 0, 0, 0.20);
    --glass-shadow-md: 0 4px 16px rgba(0, 0, 0, 0.30);
    --glass-shadow-lg: 0 8px 32px rgba(0, 0, 0, 0.40);
    --glass-shadow-xl: 0 16px 48px rgba(0, 0, 0, 0.50);
    --glass-shadow-inset: inset 0 1px 0 rgba(255, 255, 255, 0.06);

    --text-primary: rgba(255, 255, 255, 0.90);
    --text-secondary: rgba(255, 255, 255, 0.60);
    --text-tertiary: rgba(255, 255, 255, 0.35);
    --text-on-glass: rgba(255, 255, 255, 0.85);
    --text-on-glass-secondary: rgba(255, 255, 255, 0.55);
  }
}
```

---

## 第三章 · 玻璃效果实现

### 3.1 四层渲染模型

每个玻璃元素由四个视觉层叠加构成，缺一不可：

```
┌──────────────────────────────────────────┐
│  第 4 层: 环境阴影（Ambient Shadow）       │  → box-shadow
│  ┌──────────────────────────────────────┐ │
│  │  第 3 层: 表面反射（Surface Shine）   │ │  → border + inset shadow
│  │  ┌──────────────────────────────────┐│ │
│  │  │  第 2 层: 折射模糊（Refraction） ││ │  → backdrop-filter: blur()
│  │  │  ┌──────────────────────────────┐││ │
│  │  │  │  第 1 层: 半透明底色         │││ │  → background: rgba()
│  │  │  │  (Translucent Base)          │││ │
│  │  │  └──────────────────────────────┘││ │
│  │  └──────────────────────────────────┘│ │
│  └──────────────────────────────────────┘ │
└──────────────────────────────────────────┘
```

### 3.2 标准玻璃组件 CSS

```css
/* 标准玻璃面板 */
.glass-panel {
  background: var(--glass-bg-regular);
  backdrop-filter: blur(var(--glass-blur-lg));
  -webkit-backdrop-filter: blur(var(--glass-blur-lg));
  border: 1px solid var(--glass-border-regular);
  border-top-color: var(--glass-border-accent-top);
  border-radius: var(--radius-xl);
  box-shadow: 
    var(--glass-shadow-md),
    var(--glass-shadow-inset);
}

/* 侧边栏玻璃 */
.glass-sidebar {
  background: var(--glass-bg-thin);
  backdrop-filter: blur(var(--glass-blur-md));
  -webkit-backdrop-filter: blur(var(--glass-blur-md));
  border-right: 1px solid var(--glass-border-subtle);
}

/* 工具栏玻璃 */
.glass-toolbar {
  background: var(--glass-bg-regular);
  backdrop-filter: blur(var(--glass-blur-xl));
  -webkit-backdrop-filter: blur(var(--glass-blur-xl));
  border-bottom: 1px solid var(--glass-border-regular);
  box-shadow: var(--glass-shadow-sm);
}

/* 悬浮卡片 */
.glass-card-elevated {
  background: var(--glass-bg-thick);
  backdrop-filter: blur(var(--glass-blur-xl));
  -webkit-backdrop-filter: blur(var(--glass-blur-xl));
  border: 1px solid var(--glass-border-strong);
  border-top-color: var(--glass-border-accent-top);
  border-radius: var(--radius-2xl);
  box-shadow: 
    var(--glass-shadow-lg),
    var(--glass-shadow-inset);
}

/* 弹出菜单/下拉框 */
.glass-popover {
  background: var(--glass-bg-ultra-thick);
  backdrop-filter: blur(var(--glass-blur-2xl));
  -webkit-backdrop-filter: blur(var(--glass-blur-2xl));
  border: 1px solid var(--glass-border-strong);
  border-radius: var(--radius-xl);
  box-shadow: var(--glass-shadow-xl);
}
```

### 3.3 Tailwind CSS 扩展配置

```typescript
// tailwind.config.ts 中的扩展
export default {
  theme: {
    extend: {
      backdropBlur: {
        xs: "4px",
        glass: "16px",
        "glass-heavy": "24px",
        "glass-ultra": "32px",
      },
      borderRadius: {
        glass: "16px",
        "glass-lg": "20px",
        "glass-xl": "24px",
      },
      boxShadow: {
        glass: "0 4px 16px rgba(0, 0, 0, 0.10), inset 0 1px 0 rgba(255, 255, 255, 0.10)",
        "glass-lg": "0 8px 32px rgba(0, 0, 0, 0.15), inset 0 1px 0 rgba(255, 255, 255, 0.10)",
        "glass-xl": "0 16px 48px rgba(0, 0, 0, 0.20)",
      },
      colors: {
        brand: {
          navy: {
            DEFAULT: "#1F456E",
            light: "#2A5A8F",
            dark: "#152F4D",
          },
          gold: {
            DEFAULT: "#FFC000",
            light: "#FFD54F",
            dark: "#E5AC00",
          },
        },
        glass: {
          white: {
            3: "rgba(255, 255, 255, 0.03)",
            6: "rgba(255, 255, 255, 0.06)",
            10: "rgba(255, 255, 255, 0.10)",
            12: "rgba(255, 255, 255, 0.12)",
            15: "rgba(255, 255, 255, 0.15)",
            18: "rgba(255, 255, 255, 0.18)",
            25: "rgba(255, 255, 255, 0.25)",
          },
        },
      },
    },
  },
};
```

### 3.4 关键实现原则

**边框比表面更重要：** 在 Web 端模拟 Liquid Glass 时，精心设计的渐变边框（模拟光线在边缘的交互）比调整表面透明度能创造更令人信服的玻璃错觉。顶部边框应始终比其他边框更亮。

**避免过度使用：** Apple 官方文档明确指出——Liquid Glass 效果应该**谨慎使用**，仅限于最重要的功能性元素。过度使用会分散用户对内容的注意力。在 NCdesktop 中，只有以下元素应使用玻璃效果：
- 侧边栏
- 工具栏 / 标题栏
- 弹出菜单和模态框
- 悬浮操作按钮

**内容区域保持纯净：** 编辑器和笔记内容区域不应使用玻璃效果，应保持清晰的纯色或极轻微的半透明背景。

---

## 第四章 · 颜色系统

### 4.1 品牌色在玻璃上的应用

品牌色不应大面积涂抹在玻璃材质上，而是作为**点睛之笔**出现：

| 场景 | 颜色使用 |
|------|----------|
| 活跃/选中状态 | 品牌海军蓝 `#1F456E` 低透明度叠加 |
| 强调操作按钮 | 品牌金色 `#FFC000` 作为填充或边框 |
| 侧边栏选中项 | `rgba(31, 69, 110, 0.12)` 背景 + 左侧 2px 金色指示条 |
| 标签着色 | 使用品牌色的淡化版本 |

### 4.2 系统色参考

遵循 macOS 系统色彩，确保与 Liquid Glass 材质和谐共存：

| 语义 | 亮色模式 | 暗色模式 |
|------|----------|----------|
| 系统蓝 | `#007AFF` | `#0A84FF` |
| 系统绿 | `#34C759` | `#30D158` |
| 系统橙 | `#FF9500` | `#FF9F0A` |
| 系统红 | `#FF3B30` | `#FF453A` |
| 系统青 | `#5AC8FA` | `#64D2FF` |
| 系统紫 | `#AF52DE` | `#BF5AF2` |

### 4.3 色彩使用量化规则

```
玻璃材质颜色分配（60-30-10 法则的变体）：

70% — 透明/半透明（让内容和桌面壁纸透出）
20% — 中性色文本和图标
7%  — 系统灰色（分割线、次要边框）
3%  — 品牌色强调（选中态、关键操作）
```

---

## 第五章 · 排版系统

### 5.1 字体栈

```css
/* 界面文字 */
font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", 
             "SF Pro Display", "PingFang SC", "Microsoft YaHei", sans-serif;

/* 代码/等宽 */
font-family: "SF Mono", "Fira Code", "JetBrains Mono", Menlo, monospace;
```

### 5.2 字号阶梯

基于 macOS Human Interface Guidelines 的动态类型比例：

| 级别 | 尺寸 | 行高 | 字重 | 用途 |
|------|------|------|------|------|
| Caption 2 | 11px | 1.3 | Regular (400) | 时间戳、辅助信息 |
| Caption 1 | 12px | 1.35 | Regular (400) | 标签、状态文字 |
| Footnote | 13px | 1.4 | Regular (400) | 侧边栏次要文字、工具提示 |
| Body | 15px | 1.5 | Regular (400) | 正文、列表项、编辑器默认 |
| Callout | 16px | 1.45 | Regular (400) | 卡片标题、表单标签 |
| Headline | 17px | 1.3 | Semibold (600) | 区块标题、导航标题 |
| Title 3 | 20px | 1.25 | Semibold (600) | 页面副标题 |
| Title 2 | 22px | 1.2 | Bold (700) | 页面标题 |
| Title 1 | 28px | 1.15 | Bold (700) | 大标题 |
| Large Title | 34px | 1.1 | Bold (700) | 欢迎页、空状态 |

### 5.3 排版在玻璃上的增强

在半透明玻璃背景上，文字需要额外的可读性处理：

```css
/* 玻璃面上的文字增强 */
.text-on-glass {
  color: var(--text-on-glass);
  text-shadow: 0 0.5px 1px rgba(0, 0, 0, 0.08);
  -webkit-font-smoothing: antialiased;
  font-weight: 500; /* Liquid Glass 推荐使用略粗的字重 */
}

/* 暗色模式下的玻璃文字 */
[data-theme="dark"] .text-on-glass {
  text-shadow: 0 0.5px 1px rgba(0, 0, 0, 0.25);
}
```

---

## 第六章 · 形状系统

### 6.1 同心圆角（Concentric Shapes）

Apple Liquid Glass 引入了"同心形状"概念：嵌套元素的内圆角 = 外圆角 - 内外间距，确保视觉上的几何和谐。

```
外容器圆角: 20px
容器内边距: 8px
内元素圆角: 20px - 8px = 12px

┌────────────────────────────┐  ← 外圆角 20px
│  ┌──────────────────────┐  │
│  │                      │  │  ← 内圆角 12px
│  │    内容区域           │  │
│  │                      │  │
│  └──────────────────────┘  │
│         8px 间距            │
└────────────────────────────┘
```

### 6.2 三种形状类型

| 类型 | 特征 | 用途 |
|------|------|------|
| **固定形状** | 恒定圆角值 | 基础布局容器、面板 |
| **胶囊形状** | `border-radius: 9999px` | 大号/特大号控件、标签、醒目按钮 |
| **同心形状** | 内圆角 = 外圆角 - padding | 所有嵌套的容器和控件 |

### 6.3 控件尺寸规格

| 尺寸 | 高度 | 圆角 | 内边距(h) | 字号 | 用途 |
|------|------|------|-----------|------|------|
| XS | 24px | 6px | 8px | 11px | 紧凑工具栏项、标签 |
| SM | 28px | 8px | 10px | 13px | 次要按钮、筛选器 |
| MD | 34px | 10px | 14px | 15px | 标准按钮、输入框（默认） |
| LG | 40px | 12px | 18px | 17px | 主要操作按钮 |
| XL | 48px | 16px | 22px | 17px | 特大操作、胶囊按钮 |

---

## 第七章 · 交互动效

### 7.1 动画原则

Liquid Glass 的动效追求**自然流体感**——像水一样流动，像玻璃一样柔和反射。不使用生硬的线性动画，一切运动都有呼吸感。

### 7.2 过渡时间规范

| 类别 | 时长 | 缓动函数 | 适用场景 |
|------|------|----------|----------|
| 微交互 | 100-150ms | `ease-out` | hover 状态、焦点指示 |
| 标准过渡 | 200-300ms | `cubic-bezier(0.16, 1, 0.3, 1)` | 面板展开、选项切换 |
| 中等动画 | 300-500ms | `cubic-bezier(0.25, 1, 0.5, 1)` | 侧边栏滑入、模态弹出 |
| 大型变换 | 500-800ms | `cubic-bezier(0.76, 0, 0.24, 1)` | 页面转场、布局重排 |
| 弹性效果 | 400-600ms | `cubic-bezier(0.34, 1.56, 0.64, 1)` | 按钮按压回弹、拖拽释放 |

### 7.3 标准交互动效

```css
/* 悬浮效果：玻璃元素微微提升 */
.glass-interactive {
  transition: 
    transform var(--duration-fast) var(--ease-out-expo),
    box-shadow var(--duration-fast) var(--ease-out-expo),
    background-color var(--duration-fast) var(--ease-out-expo);
}

.glass-interactive:hover {
  transform: translateY(-1px);
  box-shadow: var(--glass-shadow-lg);
  background: var(--glass-bg-thick);
}

.glass-interactive:active {
  transform: translateY(0) scale(0.98);
  box-shadow: var(--glass-shadow-sm);
  transition-duration: var(--duration-instant);
}

/* 侧边栏项选中动效 */
.sidebar-item {
  transition: 
    background-color var(--duration-fast) var(--ease-out-expo),
    padding-left var(--duration-normal) var(--ease-spring);
}

.sidebar-item.active {
  background: rgba(31, 69, 110, 0.12);
  padding-left: calc(var(--space-4) + 2px);
}

/* 模态框入场动效 */
@keyframes glass-modal-enter {
  from {
    opacity: 0;
    transform: scale(0.95) translateY(8px);
    backdrop-filter: blur(0px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
    backdrop-filter: blur(var(--glass-blur-2xl));
  }
}

.glass-modal-enter {
  animation: glass-modal-enter var(--duration-normal) var(--ease-out-expo);
}
```

### 7.4 形态变换（Morphing）

Liquid Glass 的标志性特征是控件之间的流体变换——按钮膨胀为菜单，标签页平滑滑动等。在 Web 端通过 CSS View Transitions 或 FLIP 动画实现：

```css
/* 使用 View Transitions API 实现形态变换 */
::view-transition-old(glass-element),
::view-transition-new(glass-element) {
  animation-duration: var(--duration-normal);
  animation-timing-function: var(--ease-out-expo);
}
```

---

## 第八章 · 布局系统

### 8.1 NCdesktop 主界面布局

```
┌────────────────────────────────────────────────────────────┐
│ ● ● ●  ┃          标题栏 / 工具栏 (Glass Toolbar)        │
│ 窗口控件 ┃  [搜索] [+新建]           [视图] [设置]         │
├──────────╋─────────────────────┬──────────────────────────┤
│          ┃                     │                          │
│  侧边栏  ┃     笔记列表         │       编辑器区域         │
│  (Glass  ┃     (内容区域)       │       (纯净背景)         │
│  Sidebar)┃                     │                          │
│          ┃  ┌───────────────┐  │  ┌──────────────────┐   │
│  📁 全部  ┃  │ 笔记卡片 1     │  │  │  # 笔记标题       │   │
│  ⭐ 收藏  ┃  │ 预览文本...    │  │  │                  │   │
│  🗓 最近  ┃  └───────────────┘  │  │  正文内容...      │   │
│  🏷 标签  ┃  ┌───────────────┐  │  │                  │   │
│  📂 文件夹┃  │ 笔记卡片 2     │  │  │                  │   │
│          ┃  │ 预览文本...    │  │  └──────────────────┘   │
│          ┃  └───────────────┘  │                          │
│          ┃                     │                          │
└──────────╩─────────────────────┴──────────────────────────┘
```

### 8.2 响应式窗口尺寸

| 窗口宽度 | 布局策略 |
|----------|----------|
| ≥ 1200px | 三栏：侧边栏(220px) + 列表(300px) + 编辑器(flex) |
| 900-1199px | 三栏压缩：侧边栏(180px) + 列表(260px) + 编辑器(flex) |
| 700-899px | 两栏：侧边栏隐藏为图标栏(56px) + 列表/编辑器(flex) |
| < 700px | 单栏：堆叠式导航，类似移动端 |

### 8.3 间距规范

基于 **4px 基准网格**，所有间距必须是 4 的倍数：

| 场景 | 间距值 |
|------|--------|
| 组件内部紧凑间距 | 4px (--space-1) |
| 相关元素间 | 8px (--space-2) |
| 表单字段间 | 12px (--space-3) |
| 卡片内边距 | 16px (--space-4) |
| 区块之间 | 24px (--space-6) |
| 页面边距 | 32px (--space-8) |
| 大区块隔离 | 48px (--space-12) |

---

## 第九章 · 组件规范

### 9.1 玻璃按钮

```css
/* 标准玻璃按钮 */
.btn-glass {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  height: 34px;
  padding: 0 var(--space-4);
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-on-glass);
  background: var(--glass-bg-regular);
  backdrop-filter: blur(var(--glass-blur-lg));
  -webkit-backdrop-filter: blur(var(--glass-blur-lg));
  border: 1px solid var(--glass-border-regular);
  border-radius: var(--radius-md);
  box-shadow: var(--glass-shadow-sm), var(--glass-shadow-inset);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out-expo);
  user-select: none;
}

.btn-glass:hover {
  background: var(--glass-bg-thick);
  box-shadow: var(--glass-shadow-md), var(--glass-shadow-inset);
  transform: translateY(-0.5px);
}

.btn-glass:active {
  transform: scale(0.97);
  box-shadow: var(--glass-shadow-sm);
  transition-duration: var(--duration-instant);
}

/* 品牌强调按钮 */
.btn-glass-accent {
  background: rgba(255, 192, 0, 0.18);
  border-color: rgba(255, 192, 0, 0.30);
  color: var(--brand-navy);
}
```

### 9.2 玻璃输入框

```css
.input-glass {
  width: 100%;
  height: 34px;
  padding: 0 var(--space-3);
  font-size: var(--text-base);
  color: var(--text-primary);
  background: var(--glass-bg-thin);
  backdrop-filter: blur(var(--glass-blur-sm));
  -webkit-backdrop-filter: blur(var(--glass-blur-sm));
  border: 1px solid var(--glass-border-subtle);
  border-radius: var(--radius-md);
  outline: none;
  transition: all var(--duration-fast) var(--ease-out-expo);
}

.input-glass:focus {
  background: var(--glass-bg-regular);
  border-color: var(--brand-navy);
  box-shadow: 0 0 0 3px rgba(31, 69, 110, 0.15);
}

.input-glass::placeholder {
  color: var(--text-tertiary);
}
```

### 9.3 侧边栏列表项

```css
.sidebar-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-on-glass);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-out-expo);
  position: relative;
}

.sidebar-item:hover {
  background: var(--glass-bg-thin);
}

.sidebar-item.active {
  background: rgba(31, 69, 110, 0.12);
  font-weight: 600;
}

.sidebar-item.active::before {
  content: "";
  position: absolute;
  left: 0;
  top: 6px;
  bottom: 6px;
  width: 2.5px;
  background: var(--brand-gold);
  border-radius: var(--radius-full);
}
```

---

## 第十章 · 窗口效果与原生集成

### 10.1 Tauri Liquid Glass 窗口

通过 `tauri-plugin-liquid-glass` 实现原生窗口级玻璃效果：

```json
// tauri.conf.json
{
  "app": {
    "windows": [
      {
        "title": "NoteCapt",
        "width": 1200,
        "height": 800,
        "minWidth": 600,
        "minHeight": 400,
        "transparent": true,
        "decorations": true,
        "titleBarStyle": "Overlay"
      }
    ]
  },
  "plugins": {
    "liquid-glass": {
      "cornerRadius": 12,
      "tintColor": "#1F456E10"
    }
  }
}
```

### 10.2 CSS 全局透明基础

```css
/* 窗口透明基础 — 必须设置 */
html, body, #root {
  background: transparent;
  margin: 0;
  padding: 0;
  height: 100%;
}

/* macOS 标题栏拖拽区域 */
.titlebar-drag-region {
  -webkit-app-region: drag;
  height: 52px;
}

.titlebar-drag-region button,
.titlebar-drag-region input {
  -webkit-app-region: no-drag;
}
```

### 10.3 向下兼容策略

当系统不支持 Liquid Glass 或用户开启了"降低透明度"时的回退方案：

```css
/* 回退策略 */
@media (prefers-reduced-transparency: reduce) {
  .glass-panel,
  .glass-sidebar,
  .glass-toolbar {
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
    background: var(--fallback-bg-solid, #f5f5f7);
    border-color: var(--fallback-border, rgba(0, 0, 0, 0.12));
  }
}

/* 不支持 backdrop-filter 的回退 */
@supports not (backdrop-filter: blur(1px)) {
  .glass-panel {
    background: rgba(245, 245, 247, 0.95);
  }
}
```

---

## 第十一章 · 无障碍（Accessibility）

### 11.1 对比度要求

| 元素 | 最低对比度（WCAG 2.1） |
|------|------------------------|
| 正文文本（≥15px） | 4.5:1（AA 级） |
| 大字文本（≥18px Bold） | 3:1（AA 级） |
| 交互元素边界 | 3:1 |
| 图标（独立传达信息的） | 3:1 |

### 11.2 动效偏好适配

```css
@media (prefers-reduced-motion: reduce) {
  * {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

### 11.3 焦点指示器

每个可交互的玻璃元素必须在键盘导航时显示清晰的焦点环：

```css
.glass-interactive:focus-visible {
  outline: 2px solid var(--brand-navy);
  outline-offset: 2px;
  box-shadow: 0 0 0 4px rgba(31, 69, 110, 0.20);
}
```

### 11.4 屏幕阅读器

- 所有图标按钮必须有 `aria-label`
- 侧边栏导航使用 `<nav aria-label="主导航">`
- 玻璃面板使用语义化 `<section>` 或 `<aside>`
- 模态框使用 `role="dialog"` 和 `aria-modal="true"`

---

## 第十二章 · 性能优化

### 12.1 GPU 渲染性能

`backdrop-filter` 是 GPU 密集型操作，需要严格控制：

| 规则 | 说明 |
|------|------|
| 限制模糊面积 | 避免对超过屏幕 30% 面积的元素使用高模糊值 |
| 限制嵌套层数 | 最多 2 层叠加的 backdrop-filter |
| 避免动画模糊值 | 不要在动画中改变 blur() 的值 |
| 使用 will-change | 对频繁动画的玻璃元素添加 `will-change: transform` |
| 控制刷新 | 滚动时的玻璃元素使用 `content-visibility: auto` |

### 12.2 性能目标

- 所有玻璃效果动画保持 **60fps**
- 首次内容绘制（FCP）< **200ms**（Tauri WebView 内）
- 滚动时无视觉卡顿或撕裂

---

## 第十三章 · 暗色模式

### 13.1 模式切换策略

NCdesktop 支持三种主题模式：

| 模式 | 行为 |
|------|------|
| `system` | 跟随 macOS 系统设置（默认） |
| `light` | 强制亮色 |
| `dark` | 强制暗色 |

### 13.2 暗色模式下的玻璃差异

暗色模式下玻璃效果需要调整：

- **背景更暗、更不透明**：暗色模式下的玻璃背景使用更深的基色和稍高的不透明度，避免过度透出杂乱的暗色内容
- **边框更微妙**：使用更低透明度的白色边框，避免在暗背景上过于刺眼
- **阴影更浓重**：暗色模式下阴影需要更高的不透明度才能产生可见的深度感
- **文字反色**：白色文字需要轻微的暗色 text-shadow 增强可读性

---

## 附录 · 设计检查清单

在每个 UI 组件或页面完成时，对照以下清单验证：

- [ ] **材质正确性**: 使用了正确层级的玻璃材质？
- [ ] **四层完整性**: 半透明底色 + 模糊 + 边框反射 + 阴影，四层都到位？
- [ ] **同心圆角**: 嵌套元素的圆角遵循同心规则？
- [ ] **颜色克制**: 品牌色只用于点睛，没有大面积铺开？
- [ ] **文字可读**: 玻璃上的文字在各种背景下都清晰可读？
- [ ] **间距规范**: 所有间距都是 4px 的倍数？
- [ ] **动效自然**: 过渡使用了正确的时长和缓动函数？
- [ ] **暗色适配**: 在暗色模式下检查过外观？
- [ ] **降低透明度**: 在"降低透明度"设置下检查过？
- [ ] **减少动效**: 在"减少动效"设置下检查过？
- [ ] **键盘导航**: 焦点指示器清晰可见？
- [ ] **性能达标**: 动画保持 60fps，无卡顿？
