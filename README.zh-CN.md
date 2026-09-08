# A3S TUI
<p align="center">
  <strong>Language / 语言:</strong>
  <a href="README.md">English</a> ·
  <a href="README.zh-CN.md">中文</a>
</p>

**终端用户界面的 TEA（The Elm Architecture）框架**

A3S TUI 是一个 Rust 库，用于使用 Elm 架构模式构建终端应用程序。它将声明式 UI 与 Flexbox 布局、增量渲染和丰富的组件库结合在一起。

[![crates.io](https://img.shields.io/crates/v/a3s-tui)](https://crates.io/crates/a3s-tui)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

---

## 为什么

大多数终端 UI 库强制您手动管理状态、布局和渲染。 A3S TUI 为终端带来了现代 UI 模式：

- **TEA 架构** — 使用模型-更新-视图进行可预测的状态管理
- **声明式 UI** — 描述您想要什么，而不是如何绘制它
- **Flexbox 布局** — 由 [Taffy](https://github.com/DioxusLabs/taffy) 提供支持的类 CSS 布局
- **增量渲染** - 只重绘发生变化的部分
- **丰富的组件** — 60 个即用型组件（表格、模式、帮助面板、文本编辑器等）
- **终端原生 Markdown** — 可点击的 OSC 8 链接、响应式表格、显示宽度安全换行和受保护的多色代码突出显示

---

## 快速开始

添加到`Cargo.toml`：

```toml
[dependencies]
a3s-tui = "0.1"
tokio = { version = "1", features = ["full"] }
```

创建一个计数器应用程序：

```rust
use a3s_tui::prelude::*;
use a3s_tui::{col, text};

struct Counter { count: i64 }

enum Msg {
    Increment,
    Decrement,
    Quit,
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match event {
            Event::Key(key) if key.code == KeyCode::Up => Msg::Increment,
            Event::Key(key) if key.code == KeyCode::Down => Msg::Decrement,
            Event::Key(key) if key.code == KeyCode::Char('q') => Msg::Quit,
            _ => Msg::Increment, // fallback
        }
    }
}

impl ElementModel for Counter {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Option<cmd::Cmd<Msg>> {
        match msg {
            Msg::Increment => { self.count += 1; None }
            Msg::Decrement => { self.count -= 1; None }
            Msg::Quit => Some(cmd::quit()),
        }
    }

    fn view(&self) -> Element<Msg> {
        col![
            text!(""),
            Element::Text(
                TextElement::new(format!("Counter: {}", self.count))
                    .bold()
                    .fg(Color::Cyan)
            ),
            text!(""),
            text!("Up/Down to change | q to quit").dim(),
        ]
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    ElementProgramBuilder::new(Counter { count: 0 })
        .with_alt_screen()
        .with_fps(30)
        .run()
        .await
}
```

使用 `cargo run --example counter_element` 运行。

---

## API 接口

对于应用程序代码，更喜欢从稳定的 prelude 导入：

```rust
use a3s_tui::prelude::*;
use a3s_tui::{col, row, text};
```

Prelude 包含 TEA 程序构建器、事件类型、布局基元、
元素类型、样式类型、输入路由、键盘映射、焦点助手和
`components`模块。它还导出`AgentChrome`，一个小型中间件构建器
用于跨记录、输入、状态、任务、帮助、差异应用共享主题，
日志和工具日志表面。低层模块如`paint`、`renderer`、
`layout_engine`，各个组件模块对于高级功能仍然是公开的
使用，但前奏是 semver 稳定的应用程序代码的预期起点。

交互式列表式组件还实现了小型共享状态特征：

|特质 |目的|
| ---| ---|
| `Selectable` |读取项目计数，读取所选索引，移动到第一个/上一个/下一个/最后一个项目，并选择带有夹紧的项目。 |
| `Scrollable` |读取并设置组件滚动偏移量，滚动带符号的增量，并跳转到组件拥有的边界的顶部/底部。 |
| `Tabbed` |读取标签计数，读取活动标签，并通过夹紧切换第一个/上一个/下一个/最后一个标签。 |
| `Activatable` |检查所选项目是否可以发出操作，包括禁用的菜单行。 |

这些特征不会取代特定于组件的消息枚举，例如
`MenuPanelMsg`或`DataTableMsg`；他们为应用程序外壳和命令系统提供了
跨组件协调选择、滚动和选项卡状态的常用方法。

```rust
use a3s_tui::prelude::*;

fn move_down<T: Selectable>(component: &mut T) {
    component.select_next();
}

fn maybe_run<T: Activatable>(component: &T) {
    if component.can_activate_selected() {
        // Dispatch the component-specific selected action.
    }
}
```

对于应用程序级别的输入编排，使用`InputRouter`来解析固定的键
顺序：最新捕获的范围，焦点组件绑定，然后是全局绑定。
这会阻止模态、直通覆盖和常规聚焦小部件
每个手卷都有不同的快捷策略。

```rust
use a3s_tui::prelude::*;

const PROMPT: FocusId = 1;

#[derive(Clone)]
enum Action {
    ClosePalette,
    PromptSubmit,
    Quit,
}

let mut focus = FocusManager::new();
focus.register(PROMPT);

let mut router = InputRouter::new()
    .bind_global(KeyBinding::ctrl(KeyCode::Char('c')), Action::Quit, "Quit")
    .bind_focus(PROMPT, KeyBinding::new(KeyCode::Enter), Action::PromptSubmit, "Submit prompt")
    .bind_scope("palette", KeyBinding::new(KeyCode::Esc), Action::ClosePalette, "Close palette");

router.push_capture("palette");

if let Event::Key(key) = event {
    if let Some(routed) = router.resolve_key(&key, focus.current()) {
        // Dispatch routed.action in your update function.
    }
}
```

主题感知应用程序可以使用稳定的语义标记，而不是存储原始的
应用程序状态下的颜色。内置主题具有易于配置的名称，并且
`ThemeRole` 将设计系统角色映射到具体的 `Color` 和 `Style` 值：

```rust
use a3s_tui::prelude::*;

let theme = Theme::from_builtin_name("tokyo-night").unwrap_or_default();

let title = TextElement::new("Workspace")
    .bold()
    .fg(theme.color(ThemeRole::Primary));

let selected = theme.selection_style().render("  current row");
let panel = theme.surface_style().render("Connected");

let menu = components::MenuPanel::new("Command palette")
    .item(components::MenuItem::new("/theme"))
    .with_theme(&theme);

let table = components::DataTable::new(vec![components::DataColumn::new("Name")])
    .with_theme(&theme);

let chrome = AgentChrome::new(&theme);

let transcript = chrome
    .output("Ran command")
    .line("tests passed")
    .view(80);

let prompt = chrome
    .prompt("❯ ")
    .text("/model gpt-5")
    .view();

let footer = chrome
    .session_status("/workspace/a3s")
    .model("openai/gpt-5")
    .context(42_000, 128_000)
    .view(80);

let plan = chrome
    .checklist(vec![
        chrome.checklist_item("collect evidence").done(),
        chrome.checklist_item("verify patch").active(),
    ])
    .view(80, 4);

let diff = chrome
    .diff_texts("src/lib.rs", "old\n", "new\n")
    .view(80, 8);

for preset in Theme::builtins() {
    println!("{} -> {}", preset.name(), preset.label());
}
```

## 功能标志

默认功能保留完整的当前体验：

```toml
a3s-tui = "0.1"
```

对于不渲染 Markdown 转录本或突出显示的轻量级应用程序
代码块，禁用默认值：

```toml
a3s-tui = { version = "0.1", default-features = false }
```

可用功能：

|特色 |默认 |启用|
| --- | --- | --- |
| `markdown` |是的 | `a3s_tui::markdown::Markdown` 和 `a3s_tui::streaming::StreamingMarkdown` 通过 `comrak`。 |
| `syntax-highlighting` |是的 | `syntect` Markdown 代码块的代码突出显示。需要`markdown`。 |
| `full` |没有 | `markdown` + `syntax-highlighting` 的便捷别名。 |

关闭 `syntax-highlighting` 保持 Markdown 渲染可用，但渲染
代码块内容为纯文本。

公认的围栏代码语言使用仅前景语法颜色。未知
大于 512 KiB 或 10,000 行的语言标签和块保持无样式，因此
流媒体转录仍然是可预测的和响应性的。

---

## 特征

### 架构

- **TEA 模式** — 具有不可变状态的模型-更新-视图循环
- **元素树** — 用于声明式 UI 的类似虚拟 DOM 的树结构
- **Taffy Flexbox** — CSS Flexbox 布局引擎（flex-direction、gap、padding、align-items、justify-content）
- **增量渲染** - 线差异算法最大限度地减少终端重绘
- **异步运行时** — 由 Tokio 提供支持的非阻塞事件循环

### 成分

|组件|描述 |
|------------|-------------|
| `ActivityBlock` |具有时尚细节和可选实时输出尾部的飞行活动线 |
| `Alert` |带有字符串和元素渲染的彩色警报（成功/信息/警告/错误）|
| `Badge` |带有字符串和元素渲染的内联状态徽章 |
| `Breadcrumb` |分层路径导航|
| `Checklist` |具有可配置字形/文本颜色的状态感知任务/TODO 列表 |
| `ChipStrip` |紧凑型彩色芯片带，具有主动芯片造型 |
| `ChoicePrompt` |用于批准和命令选择的编号操作选择器，带有滚轮/单击输入以及字符串、线条和元素渲染 |
| `Confirm` |通过内联、框和全屏渲染进行键盘/鼠标确认 |
| `ConnectorBlock` |连接器引导的紧凑输出和连续行|
| `CursorLine` |带有块光标的显示宽度感知编辑器行 |
| `DataTable` |响应式、可滚动的数据表，具有线条和元素渲染以及滚轮/单击行选择 |
| `DetailPanel` |使用元数据和操作压缩选定行的详细信息 |
| `DiffView` |用于编辑和工具输出的统一差异渲染器 |
| `Divider` |元素和线条渲染分隔符 |
| `GutterBlock` |带有标记槽和可选气泡背景的转录/消息块 |
| `HelpPanel` |分组快捷键和命令帮助 |
| `InputBorder` |带有上下文、工作量和功能区变体的输入区域边界线 |
| `InlineAction` |内联动作药丸，带有可选的静音细节文本 |
| `KeyValue` |使用字符串和元素渲染标记元数据行
| `LevelSlider` |带有刻度标签、滚轮/单击选择和选定标记的离散级别滑块 |
| `List` |带选择的滚动列表 |
| `LogView` |具有加载和空状态的可滚动日志/输出面板 |
| `MenuPanel` |带有滚动窗口、单击选择、滚轮导航和复选框切换的标题菜单 |
| `Meter` |带有可选值标签的紧凑型值表 |
| `MetricTrend` |具有趋势可视化的指标 |
| `ModeLine` |带有快捷方式提示的当前模式行|
| `MultiSelect` |带有复选框的多选列表 |
| `OutputBlock` |带有尾部预览和样式细节支持的状态标记转录/输出块 |
| `PanelFrame` |具有焦点感知边框的固定大小标题面板框架 |
| `Paragraph` |使用字符串和元素渲染进行宽度感知段落换行 |
| `PreviewPanel` |具有实时预览以及点击和滚轮处理功能的可选项目列表 |
| `Progress` |进度条 |
| `PromptLine` |带有对齐连续行的提示前缀输入文本 |
| `Scrollbar` |具有偏移量、百分比和附加到视图渲染的滚动条指示器
| `Select` |单选下拉菜单 |
| `SessionStatus` |带有上下文计量器和实时芯片的代理/会话页脚行 |
| `SectionHeader` |带有元数据和分隔行的宽度安全面板标题 |
| `ShimmerText` |活动文本的动画滑动突出显示 |
| `SideNotePanel` |紧凑型侧通道问答面板 |
| `Sparkline` |内联趋势图 |
| `SplitPane` |用于 IDE、git、内存和详细视图的两列面板 |
| `Spinner` |加载动画|
| `StatusBar` |具有可选背景的底部/标题状态栏 |
| `SubagentTracker` |并行子代理/后台工作跟踪器 |
| `Table` |带标题的数据表 |
| `Tabs` |带有元数据和每个选项卡重音的选项卡导航
| `TabbedMenuPanel` |带有鼠标切换和滚动感知选定列表的彩色选项卡条 |
| `TaskQueue` |固定正在运行和排队的任务面板|
| `TextInput` |单行文本输入|
| `TextOverlay` |将瞬态覆盖行组合到渲染的文本框架中 |
| `Textarea` |带滚动功能的多行文本编辑器 |
| `Timeline` |带有彩色节点和选定行突出显示的分段时间线 |
| `ToolLogView` |带有参数和缩进输出的完整工具/命令历史记录 |
| `ToolStatusLine` |带有标记、详细信息和后缀的单行工​​具状态 |
| `Toast` / `ToastManager` |带有字符串和元素渲染的瞬态通知 |
| `Tree` |可扩展的树视图 |
| `TreePicker` |具有点击和滚轮处理功能的可选择文件和层次结构选择器 |
| `Modal` |覆盖对话框|
| `Viewport` |具有可重复使用的文本选择助手的可滚动内容容器 |
| `WelcomeBanner` |首次运行的吉祥物/艺术横幅，包含元数据、提示和通知 |
| `WrappedPrefixBlock` |带有对齐延续前缀的包装标注/转录块 |

### 组件使用指南

大多数组件都故意做得很小。通过组合一些内容来构建屏幕
专门构建的组件，而不是创建一个大型渲染器。组件
通常遵循以下一种或多种形状：

- **元素组件**返回`Element<Msg>`并参与Flexbox布局。
- **行组件** 返回 `String` 或 `Vec<String>` 用于转录、覆盖、
  和固定格式渲染。
- **交互式组件**公开 `handle_key` 和/或 `handle_mouse` 助手
  并返回一个小消息枚举，例如 `MenuPanelMsg`、`DataTableMsg`，或者
  `ChoicePromptMsg`。

#### 导航和选择

|组件|用它来 |典型用法|
| ---| ---| ---|
| `Tabs` |紧凑的水平选项卡行，其中每个选项卡都会更改一个视图。 |保持活动选项卡处于应用状态；启用捕获时调用选项卡鼠标处理程序。 |
| `TabbedMenuPanel` |帐户/模型选择器和其他选项卡式命令菜单。 |使用 `TabbedMenuTab` 构建选项卡；为提供商身份启用`inactive_tabs_use_tab_color`，并在明亮的品牌背景需要深色标签时设置`active_tab_foreground`。 |
| `MenuPanel` |斜杠菜单、资源选择器、插件切换和简短的覆盖菜单。 |创建`MenuItem`行，设置`selected`和`scroll`，渲染有界的`view`，然后处理`MenuPanelMsg`。 |
| `TreePicker` |文件选择器和扁平化层次结构选择。 |从模型构建 `TreePickerItem::branch` 和 `TreePickerItem::leaf` 行；处理打开、关闭、选择和取消的消息。 |
| `Select` |小型单选控件，其中仅选定的索引很重要。 |将选定的索引存储在应用程序状态中，并使用返回的`SelectMsg`来更新它。 |
| `MultiSelect` |类似复选框的多选列表。 |将检查的索引与光标状态分开存储，并从 `MultiSelectMsg` 进行切换。 |
| `List` |简单的可滚动列表，没有丰富的行元数据。 |当标签足够并且完整的`MenuPanel`太重时使用。 |
| `ChoicePrompt` |人机交互批准提示和编号选择。 |构建 `ChoicePromptItem` 行，设置当前选择，然后将 `ChoicePromptMsg::Selected` 映射到域操作。 |
| `Confirm` |在破坏性行动之前确认是/否。 |渲染内联、盒装或全屏确认并映射`ConfirmMsg`以批准/取消行为。 |

```rust
use a3s_tui::components::{MenuItem, MenuPanel, MenuPanelMsg};

let mut menu = MenuPanel::new("Commands")
    .items(vec![
        MenuItem::new("/model").description("Switch model"),
        MenuItem::new("/theme").description("Preview themes"),
    ])
    .selected(0)
    .max_items(8);

let rendered = menu.view(64, 10).lines().collect::<Vec<_>>();

if let Some(MenuPanelMsg::Selected(index)) = menu.handle_key(&key) {
    // Run the selected command.
}
```

提供商选择器可以保留每个选项卡的品牌颜色，而无需更改
其他菜单使用的默认静音选项卡行为：

```rust
use a3s_tui::components::{TabbedMenuItem, TabbedMenuPanel, TabbedMenuTab};
use a3s_tui::style::Color;

let providers = TabbedMenuPanel::new(vec![
    TabbedMenuTab::new("Codex", Color::Rgb(16, 163, 127))
        .item(TabbedMenuItem::new("gpt-5-codex")),
    TabbedMenuTab::new("Claude", Color::Rgb(217, 119, 87))
        .item(TabbedMenuItem::new("claude-opus")),
])
.inactive_tabs_use_tab_color(true)
.active_tab_foreground(Color::Black);
```

#### 表格、时间线和结构化数据

|组件|用它来 |典型用法|
| ---| ---| ---|
| `DataTable` |响应式流程表、活动表和密集的操作视图。 |定义 `DataColumn` 宽度/优先级，添加 `DataRow` 值，然后使用 `DataTableMsg` 进行滚轮/单击行选择。 |
| `Table` |静态二维输出，无需选择或响应式隐藏。 |添加行并呈现为紧凑的只读表。 |
| `Timeline` |具有时间、状态和所选行突出显示的事件流。 |将事件保留为 `TimelineItem` 行并按时间顺序呈现历史记录、内存或运行活动。 |
| `DetailPanel` |所选项目详细信息、元数据和简短操作行。 |与列表/表格选择配对；选择更改时更新行。 |
| `KeyValue` |压缩元数据、运行时事实和摘要字段。 |为面板侧边栏和诊断添加标签/值对。 |
| `ToolLogView` |包含参数和输出的完整命令/工具历史记录。 |工具调用完成时附加 `ToolLogRecord` 条目。 |
| `LogView` |具有加载/空状态的可滚动纯输出。 |用于长日志，其中选择不如浏览重要。 |
| `Sparkline` | CPU、内存、令牌、延迟或速率的内联趋势。 |提供最近的数字样本并在表格或状态行内呈现。 |
| `MetricTrend` |一个度量值加上一个小趋势显示。 |用于数字和动作都很重要的仪表板。 |
| `Meter` |紧凑的百分比或容量指示器。 |用于上下文填充、配额、进度或运行状况计量。 |
| `Progress` |任务进度条和阶段完成情况。 |将进度存储为标准化值并在任务状态所在的位置进行渲染。 |

```rust
use a3s_tui::components::{CellAlign, DataColumn, DataRow, DataTable};

let table = DataTable::new(vec![
    DataColumn::new("PID").width(7).align(CellAlign::Right),
    DataColumn::new("CPU%").width(6).align(CellAlign::Right),
    DataColumn::new("COMMAND").min_width(16),
])
.row(DataRow::new(vec!["4242", "12.5", "a3s code"]))
.selected(Some(0))
.scroll(0);

let view = table.view(80, 12);
```

#### 转录本、代理和工具表面

|组件|用它来 |典型用法 |
| --- | --- | --- |
| `GutterBlock` |带有左标记和可选气泡样式的聊天记录条目。 |渲染助手/用户/工具块具有一致的间距；全出血文字记录换行使连续行在内容列下对齐。 |
| `PromptLine` |带提示前缀的用户输入或命令文本。 |保持连续行在提示字形下对齐。 |
| `WrappedPrefixBlock` |包装推理、标注和带前缀的转录文本。 |当每条换行必须在标记下对齐时使用。 |
| `OutputBlock` |工具输出摘要，包含状态、标题和尾部预览。 |一致地显示正在运行、已完成、失败和取消的工具输出。 |
| `ToolStatusLine` |带有标记、详细信息和后缀的一行实时刀具状态。 |用于活动工具调用的流式转录。 |
| `ActivityBlock` |带有可选标准输出尾部的实时活动行。 |显示长时间运行的工作而不占用整个屏幕。 |
| `ConnectorBlock` |带连接器字形的紧凑型多行输出。 |当工具发出视觉上应该挂在一起的相关线条时使用。 |
| `SubagentTracker` |并行子代理或后台工作跟踪。 |添加工作人员姓名、描述和状态的 `SubagentRow` 条目。 |
| `TaskQueue` |正在运行的任务加上排队的后续工作。 |在输入或状态区域附近渲染活动任务和排队任务。 |
| `Checklist` |计划、TODO 和多步骤工作流程状态。 |将每个 `ChecklistItem` 与 `ChecklistStatus` 一起存储并渲染当前计划。 |
| `InlineAction` |内联命令/操作药丸，例如“打开视图”。 |将可见标签与静音详细文本和主机端点击检测配对。 |
| `Toast` / `ToastManager` |临时通知和页脚闪烁。 |将 `Toast` 值推入管理器并在每帧渲染活动值。 |
| `SideNotePanel` |侧频道问答面板。 |用于不应成为转录历史的紧凑辅助上下文。 |
| `WelcomeBanner` |首运行或空状态欢迎面。 |启动时渲染一次，包含提示、版本元数据和通知。 |

这些组件也是 A3S Code的首选中间件构建块
图伊贝壳。在应用程序中保留 shell 拥有的状态，然后编写一个小的
将当前`Theme`传递到每个转录本、输入和状态的适配器
表面。 `AgentChrome` 还公开了模式线、任务队列的主题构建器，
子代理跟踪器、有标题或无标题的帮助面板、日志、清单、差异和
工具日志：

```rust
use a3s_tui::prelude::*;

fn render_agent_chrome(theme: &Theme, width: u16) -> String {
    let chrome = AgentChrome::new(theme);

    let status = chrome
        .status_bar()
        .left("A3S Code")
        .right("live")
        .view(width);

    let tabs = chrome
        .tabs(vec!["Chat", "Tools", "Memory"])
        .view(width);

    let output = chrome
        .output("Ran")
        .detail("cargo test")
        .line("ok")
        .view(width);

    let help = chrome
        .help_panel_without_title()
        .section(components::HelpSection::new("Keys").row("Esc", "close"))
        .view(width, 4);

    let checklist = chrome
        .checklist(vec![
            chrome.checklist_item("collect evidence").done(),
            chrome.checklist_item("verify").active(),
        ])
        .view(width, 4);

    let diff = chrome
        .diff_texts("src/lib.rs", "old\n", "new\n")
        .view(width, 4);

    let border = chrome
        .input_border()
        .context("42% context used")
        .label("◇ high")
        .view(width);

    let prompt = chrome
        .prompt("❯ ")
        .text("summarize changes")
        .view();

    [status, tabs, output, help, checklist, diff, border, prompt].join("\n")
}
```

#### 文本输入、编辑和视口

|组件|用它来 |典型用法|
| ---| ---| ---|
| `TextInput` |单行字段。 |将按键或粘贴事件转发到`TextInputMsg`；粘贴被清理为一行，并且内置了单词级导航/删除。
| `Textarea` |多行提示框和编辑器。 |配置宽度、高度、自动增长和提交行为；粘贴插入换行符而不提交，并且内置单词级导航/删除。
| `CursorLine` |使用可见光标和宽度安全文本编辑行。 |在固定宽度的编辑器面板中渲染活动行。 |
| `Viewport` |可滚动的文字记录或文档内容。 |存储视口状态并通过页面键或鼠标滚轮事件更新它； ANSI 和 OSC 8 序列在换行时不消耗可见列。 |
| `Scrollbar` |文本视图上的视觉滚动位置。 |附加到渲染视图或在固定高度面板旁边渲染。 |
| `Paragraph` |带有可选对齐方式的包裹式散文。 |用于必须适合宽度的说明、帮助文本和详细信息副本。 |
| `DiffView` |统一差异显示。 |将编辑转换为 `DiffLine` 行并使用添加/删除/上下文样式进行渲染。 |
| `Markdown` 支持 |丰富的文字记录和文档渲染。 |使用 Markdown 渲染器来显示 CommonMark 内容、代码突出显示、可点击的 OSC 8 链接以及没有原始分隔符行的窄宽度表回退。 |

#### 布局、框架和视觉结构

|组件|用它来 |典型用法|
| ---| ---| ---|
| `PanelFrame` |带有焦点感知边框的标题固定大小面板。 |环绕文件浏览器、预览和分割窗格。 |
| `SplitPane` |两栏布局，例如文件树加编辑器。 |提供左右渲染行并让组件绑定宽度。 |
| `Modal` |居中覆盖对话框。 |用于阻止应位于当前屏幕上方的对话框。 |
| `TextOverlay` |将覆盖行注入现有的渲染帧中。 |默认替换整行，或使用 `at_column` / `centered` 保留周围的框架内容。 |
| `Divider` |元素视图或线条渲染视图中的水平分隔符。 |在部分之间使用 `divider`、`divider_line` 或宽度感知变体。 |
| `SectionHeader` |带有元数据和分隔行的面板部分标题。 |使用上面分组的详细行和活动面板。 |
| `StatusBar` |具有左、中、右区域的页眉/页脚栏。 |用于屏幕标题、活动模式或面板提示。 |
| `ModeLine` |当前模式加上快捷方式提示。 |在页脚或输入区域附近渲染。 |
| `SessionStatus` |带有芯片和上下文计量器的代理/会话页脚。 |输入 cwd、分支、模型、模式和 `SessionStatusChip` 值。 |
| `InputBorder` |提示框镀铬。 |使用上下文、工作量和功能区变体渲染输入顶部/底部边框。 |
| `Breadcrumb` |路径元数据和导航上下文。 |渲染工作空间路径、配置路径或嵌套对象位置。 |
| `Badge` |内联状态标签。 |用于小型状态标记，例如“beta”、“缓存”或“远程”。 |
| `ChipStrip` |具有活跃样式的多个紧凑标签。 |根据过滤器、模式或范围的 `Chip` 值构建。 |
| `Alert` |成功、信息、警告和错误消息。 |选择 `AlertKind` 并渲染为线条或元素。 |
| `Spinner` |轻量级负载指示器。 |勾选计时器并在运行标签旁边进行渲染。 |
| `ShimmerText` |动画活动文本。 |用于旋转器太小的活动阶段。 |
| `Tree` |只读分层显示。 |当层次结构可见但不充当选择器时使用。 |

#### 常见的构图模式

当用户选择一项并且应该返回到时，使用选择器覆盖
立即当前屏幕：

```rust
let width: u16 = 80;
let frame = "...".repeat(24);
let rows = MenuPanel::new("Theme")
    .items(theme_items)
    .selected(selected)
    .max_items(10)
    .view(width, 12)
    .lines()
    .map(str::to_string)
    .collect::<Vec<_>>();

let frame = TextOverlay::new(rows)
    .bottom()
    .width(width as usize)
    .apply(&frame);
```

使用 `.at_column(column)` 仅替换固定显示中占用的单元格
列，或 `.centered()` 将最宽的覆盖行居中，同时保留
两侧的底座框架。两种模式均保留 ANSI 样式和 OSC 8 链接
未发现的内容。

当内容需要持续浏览时使用全屏面板：

```rust
let width: u16 = 80;
let height: usize = 24;
let body = DataTable::new(columns)
    .selected(Some(selected))
    .scroll(scroll)
    .view(width, height.saturating_sub(1));

let screen = format!("{}\n{}", StatusBar::new().left("/top").view(width), body);
```

### 布局和样式

- **Flexbox 布局** — `FlexDirection`、`AlignItems`、`JustifyContent`
- **尺寸** — `Auto`、`Points(f32)`、`Percent(f32)`
- **间距** — `padding`、`margin`、`gap`
- **边框** — `Single`、`Double`、`Rounded`、`Thick`
- **颜色** — 16 种 ANSI 颜色 + RGB 支持
- **文本样式** — 粗体、斜体、下划线、暗淡、删除线

### 高级功能

- **Markdown 渲染** — 可选的 CommonMark 支持，具有功能门控语法突出显示
- **流媒体内容** — 实时文本流（非常适合 LLM 输出）
- **键盘映射系统** — 类似 Vim 的按键绑定
- **焦点管理** — 组件之间的选项卡导航
- **输入路由** — 全局、集中和捕获的命令范围
- **编辑器输入** — 粘贴感知`TextInput`/`Textarea`，具有字级编辑功能
- **交互特征** — 共享 `Selectable`、`Scrollable` 和 `Tabbed` 状态合约
- **鼠标支持** — 使用组件处理程序单击、拖动和滚动事件

---

## 示例

### 组件演示

```rust
use a3s_tui::components::{Alert, AlertKind, Badge, Table, Tabs};
use a3s_tui::{col, ElementModel, ElementProgramBuilder};

struct Demo {
    tabs: Tabs,
}

impl ElementModel for Demo {
    type Msg = Msg;

    fn view(&self) -> Element<Msg> {
        col![
            self.tabs.element(),
            Alert::new(AlertKind::Success, "All systems operational.").element(),
            Badge::new("v0.1.0").color(Color::Green).element(),
            Table::new(vec!["Name", "Status"])
                .row(vec!["Server", "Online"])
                .element(),
        ]
    }
}
```

运行 `cargo run --example demo` 查看所有组件的运行情况。

### 聊天应用程序

有关完整的聊天 UI，请参阅 `examples/chat.rs`：
- 带语法高亮的 Markdown 渲染
- 流式文本输出
- 模态对话框
- 可滚动视口
- 自定义按键绑定

### 基准

运行 `cargo bench --bench rendering` 来测量热渲染路径：
- 带有 ANSI 和 CJK 文本的显示宽度助手
- `ActivityBlock`、`ChipStrip`、`ConnectorBlock`、`CursorLine`、`DataTable`、`DetailPanel`、`DiffView`、`GutterBlock`、`HelpPanel`、`InputBorder`、`LevelSlider`、 `LogView`、`MenuPanel`、`ModeLine`、`OutputBlock`、`PanelFrame`、`PromptLine`、`Scrollbar`、`SectionHeader`、`SessionStatus`、`ShimmerText`、 `SplitPane`、`StatusBar`、`SubagentTracker`、`Tabs`、`TaskQueue`、`TextOverlay`、`Timeline`、`ToolStatusLine`、`WrappedPrefixBlock` 和视口选择字符串渲染
- 混合 Markdown 渲染，包括确定性 10 KiB、100 KiB 和 1 MiB 吞吐量情况

### 集成测试

运行 `cargo test --test terminal_integration` 来练习无头终端
从元素树到 Flexbox 布局、网格绘制、ANSI 快照的管道，
调整行为大小、截断和增量差异更改。

运行 `cargo test --test render_snapshots` 来比较稳定的黄金快照
核心小部件，例如文本编辑器、菜单、树选择器和数据表。更新
这些灯具故意带有`INSTA_UPDATE=always cargo test --test render_snapshots`
检查视觉变化后。

---

## 架构

### 茶流

```text
┌─────────────────────────────────────────┐
│  User Input (keyboard, resize, etc.)    │
└──────────────────┬──────────────────────┘
                   │
                   ▼
         ┌─────────────────┐
         │  Event → Msg    │
         └────────┬────────┘
                  │
                  ▼
         ┌─────────────────┐
         │  update(msg)    │  ← Modify state
         └────────┬────────┘
                  │
                  ▼
         ┌─────────────────┐
         │  view()         │  ← Build Element tree
         └────────┬────────┘
                  │
                  ▼
         ┌─────────────────┐
         │  Layout Engine  │  ← Taffy Flexbox
         └────────┬────────┘
                  │
                  ▼
         ┌─────────────────┐
         │  Renderer       │  ← Paint to grid
         └────────┬────────┘
                  │
                  ▼
         ┌─────────────────┐
         │  Terminal       │  ← Crossterm output
         └─────────────────┘
```

### 元素树

元素是 UI 的构建块：

```rust
pub enum Element<Msg> {
    Box(BoxElement<Msg>),      // Container with Flexbox layout
    Text(TextElement),          // Styled text
    Spacer,                     // Flexible space
}
```

使用宏来实现简洁的语法：

```rust
col![                          // Vertical column
    text!("Header").bold(),
    row![                      // Horizontal row
        text!("Left"),
        Element::Spacer,       // Push to edges
        text!("Right"),
    ],
]
```

---

## API 参考

### 核心特征

#### `ElementModel`

```rust
pub trait ElementModel: Sized + 'static {
    type Msg: From<Event> + 'static;

    fn update(&mut self, msg: Self::Msg) -> Option<Cmd<Self::Msg>>;
    fn view(&self) -> Element<Self::Msg>;
}
```

### 建设者

#### `ElementProgramBuilder`

```rust
ElementProgramBuilder::new(model)
    .with_alt_screen()         // Use alternate screen buffer
    .with_fps(30)              // Target frame rate
    .with_mouse_support()      // Enable mouse events
    .run()
    .await
```

### 宏

- `col![...]` — 垂直列 (FlexDirection::Column)
- `row![...]` — 水平行 (FlexDirection::Row)
- `text!("...")` — 文本元素简写
- `spacer!()` — 柔性垫片

---

## 比较

|特色| a3s-tui |拉图伊 |草书 |
|--------|---------|---------|---------|
|架构|茶|立即模式 |面向对象|
|布局| Flexbox（太妃糖）|限制条件|线性|
|渲染|增量 |全面重绘 |增量 |
|异步 |本地人（东京）|手册|回调 |
|降价|可选内置|外部|外部|
|组件| 60+ 内置 | DIY | 10+ 内置 |

---

## 路线图

- [x] TEA架构
- [x] 元素树 + Flexbox 布局
- [x] 60 多个核心组件
- [x] 稳定应用序幕
- [x] 功能门控降价和语法突出显示
- [x] 可选择、可滚动和选项卡式组件的共享交互特征
- [x] 全局、聚焦和捕获命令范围的输入路由
- [x] Markdown 渲染
- [x] 流媒体内容
- [x] 键盘映射系统
- [x] 鼠标事件支持
- [x] 网格布局
- [x] 动画系统
- [x] 具有语义标记 API 的主题系统
- [x] 组件和核心单元测试
- [x] 性能基准
- [x] 端到端终端集成测试

---

## 贡献

欢迎贡献！请：

1.关注[Microsoft Rust Guidelines](https://microsoft.github.io/rust-guidelines)
2. 提交前运行`cargo fmt`和`cargo clippy`
3.添加新功能测试
4.更新文档

---

## 执照

MIT 许可证 - 有关详细信息，请参阅[LICENSE](LICENSE)。

---

## 致谢

- [Taffy](https://github.com/DioxusLabs/taffy) — Flexbox 布局引擎
- [Crossterm](https://github.com/crossterm-rs/crossterm) — 终端操作
- [Ink](https://github.com/vadimdemedes/ink) — 类似 React 的 TUI 框架（灵感）
- [Elm](https://elm-lang.org/) — Elm 架构模式
