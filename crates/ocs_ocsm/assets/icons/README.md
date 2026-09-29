# OCSM 功能区图标（双色扁平化 SVG）

`src/lib.rs` 通过 `include_bytes!` 把这里的 10 个 SVG 编进 `libocs_ocsm.so`
（`ICON_FRAME` … `ICON_CARD`），因此 `tools/deploy_plugin.sh` **不需要**额外
分发这些文件；改图标后重新 `cargo build --release -p ocs_ocsm` 即可。

## 统一规格

| 项 | 值 |
|----|----|
| 网格 | `viewBox="0 0 24 24"`（内容留 ~2px 内边距，约 3.5–20.5） |
| 线宽 | `1.625`（填充细节用 `stroke="none"`，几何标记 1.35） |
| 端点/拐角 | `stroke-linecap="round" stroke-linejoin="round"` |
| 主结构色 | token `#B4B6B9` → 宿主解析为**主题文字色** |
| 强调色 | token `#6DB7ED` → 宿主解析为**主题主色** |
| 风格 | 扁平、无渐变、无阴影；主结构主色 + 关键细节强调色 |

宿主渲染管线：`ocs_plugin_api::ribbon::IconKind::Svg` →
`src/ui/ribbon/widgets.rs::make_icon` → `src/ui/icons.rs::semantic()`
（按当前主题重上色，深/浅色主题都清晰）。两个 token 是 `recolor_semantic_svg`
识别的语义色，**不要**替换成任意色值，否则会失去主题适配。

## 文件 ↔ 按钮

| 文件 | 按钮 | 图形语义 |
|------|------|----------|
| `frame.svg` | 插入图幅 `OCSMFRAMEINIT` | 图框 + 角标 + 标题栏 |
| `centerline.svg` | 中心线 `OCSMCENTERLINE` | 圆 + 点划线十字 |
| `gear.svg` | 齿轮 `OCSMGEAR` | 齿形轮廓 + 中心孔 |
| `shaft.svg` | 轴生成器 `OCSMSHAFT` | 阶梯轴 + 点划线中心线 |
| `powerdim.svg` | 智能标注 `OCSMPOWERDIM` | 被测线 + 尺寸线/箭头 |
| `dimguide.svg` | 尺寸引导 `OCSMDIMGULIDE` | 虚线引导线 + 光标 |
| `dim2gb.svg` | 标注转GB `OCSMDIM2GB` | 尺寸线 + 循环箭头 |
| `parts.svg` | 标准件库 `OCSMPART` / `XL` | 六角头螺栓（头 + 杆 + 末端倒角 + 螺纹） |
| `hole.svg` | 孔生成器 `OCSMHOLE` / `DK` | 阶梯孔剖视 + 点划中心线 |
| `card.svg` | 智能卡片 `OCSMCARD` | 卡片/表格（外框 + 表头 + 参数/值两格 + 智能星标） |

按钮的 id / 显示名 / 分组 / 触发命令不随图标变化；`src/lib.rs` 的
`ribbon_tool_icons_are_two_colour_svg` 测试会校验 10 个按钮都是 24×24 双色 SVG。
