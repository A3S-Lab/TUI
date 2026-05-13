# 自动化示例录制

本目录包含自动录制 TUI 示例运行并生成 GIF 的脚本。

## 方案 1: 使用 VHS (推荐)

[VHS](https://github.com/charmbracelet/vhs) 是一个终端录制工具，可以自动化录制终端会话并生成 GIF。

### 安装 VHS

```bash
# macOS
brew install vhs

# Linux
# 参考: https://github.com/charmbracelet/vhs#installation
```

### 使用方法

```bash
# 录制所有示例
./scripts/record_examples.sh

# 录制单个示例（5秒）
./scripts/record_examples.sh counter

# 录制单个示例（自定义时长）
./scripts/record_examples.sh dashboard 10
```

生成的 GIF 将保存在 `screenshots/` 目录中。

### 自定义录制

你可以手动创建 `.tape` 文件来自定义录制：

```tape
# tapes/custom.tape
Output screenshots/custom.gif
Set FontSize 14
Set Width 1200
Set Height 800
Set Theme "Catppuccin Mocha"

Type "cargo run --example counter"
Enter
Sleep 3s
Type "q"
Sleep 1s
```

然后运行：

```bash
vhs tapes/custom.tape
```

## 方案 2: 使用 asciinema

[asciinema](https://asciinema.org/) 可以录制终端会话并生成 SVG 或 GIF。

### 安装

```bash
# macOS
brew install asciinema

# Linux
pip install asciinema
```

### 使用方法

```bash
# 录制示例
asciinema rec screenshots/counter.cast -c "cargo run --example counter"

# 转换为 GIF (需要 agg)
cargo install agg
agg screenshots/counter.cast screenshots/counter.gif
```

## 方案 3: 手动截图

如果你只需要静态截图，可以使用终端的内置截图功能：

1. 运行示例：`cargo run --example counter`
2. 使用系统截图工具（macOS: Cmd+Shift+4，Linux: gnome-screenshot）
3. 保存到 `screenshots/` 目录

## 目录结构

```
.
├── scripts/
│   └── record_examples.sh    # 自动录制脚本
├── tapes/                     # VHS tape 文件
│   ├── counter.tape
│   ├── dashboard.tape
│   └── ...
└── screenshots/               # 生成的 GIF/PNG
    ├── counter.gif
    ├── dashboard.gif
    └── ...
```

## 示例列表

当前可录制的示例：

- `counter` - 简单计数器
- `hello` - Hello World
- `spinner` - 加载动画
- `progress` - 进度条
- `input` - 文本输入
- `list` - 列表选择
- `form` - 表单验证
- `dashboard` - 系统监控仪表盘
- `todo` - 待办事项

## 提示

1. **录制时长**：根据示例复杂度调整录制时长
   - 简单示例：3-5秒
   - 交互示例：8-10秒
   - 复杂示例：10-15秒

2. **GIF 优化**：生成的 GIF 可能较大，可以使用 [gifsicle](https://www.lcdf.org/gifsicle/) 优化：
   ```bash
   gifsicle -O3 --colors 256 input.gif -o output.gif
   ```

3. **主题**：VHS 支持多种主题，可以在 `.tape` 文件中修改 `Set Theme` 行

4. **分辨率**：根据需要调整 `Set Width` 和 `Set Height`
