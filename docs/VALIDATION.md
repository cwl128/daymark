# 验证记录

## 2026-11-05 / Phase 1–2

- 在空目录建立独立 Git 仓库；包含 Tauri 2、React/TypeScript、SQLite、PRD、README、完整 Apache-2.0。
- Node 24.22.1；pnpm 11.25.0；Rust 1.99.1；macOS ARM64。Rust 安装在仓库外的本任务 work 目录，没有改动全局 PATH。
- `pnpm build` 通过（TypeScript + Vite）。
- `cargo test --locked --manifest-path src-tauri/Cargo.toml`：3 个数据库测试通过，覆盖 CRUD、完成、冲突拒绝、输入校验、设置保存、关闭重开持久化、投递去重与删除级联。
- 原生窗口启动、系统通知和跨平台发行尚待下一阶段验证。

## Phase 3–6 / 原生能力与界面

- Rust 测试 9 项通过；前端日期/跨午夜/重叠布局/表单验证测试 7 项通过。
- `pnpm tauri build --debug --bundles app` 成功生成 Daymark.app（macOS ARM64）。已实际启动，读取 `tauri://localhost` 的原生界面。
- 通过原生界面创建日程后，时间轴和下一项倒计时同步显示；打开独立的边缘侧边栏，显示同一 SQLite 日程与时间表、进度。
- 系统通知使用同步 native API 返回错误并重试，不使用桌面插件的 fire-and-forget 发送。设置不把插件返回的 granted 当成系统权限保证，而提供测试通知和系统设置提示。
- 全屏与侧栏收放、Windows 和完整发行仍在验证中。

## 浏览器回归

- Browser 插件未提供，使用 Playwright + 本机 Chrome，URL `http://127.0.0.1:1420`，1200×800。
- 7 项前端单元测试和 2 条端到端流程通过。
- 流程：新建截止任务 → 第二窗口读取 → 勾选并同步 → 刷新保留 → 改名并同步 → 周视图 7 列 → 月视图 42 格 → 删除并同步。
- 流程：非法时间显示错误并保留编辑 → 保存 15/3 分钟提醒及左侧位置 → 刷新后保留设置。
- 首轮发现选择器歧义（类型字段名称、任务在日历与清单重复），已明确可访问名称及限定检查区域；重跑通过。
- 页面标题、非空内容、无框架错误遮罩、无 pageerror、可交互均通过。该测试使用独立浏览器预览存储，不能替代 SQLite 与原生提醒验收。

## 最终本机检查与交付边界

- 最后改动后重新执行 `pnpm test`（7 通过）、`pnpm test:e2e`（2 通过）、`pnpm build`（通过）、`cargo test --locked`（9 通过）、`cargo clippy --all-targets -- -D warnings`（通过）和 Rust 格式检查。
- 界面证据：1200×800 桌面与 390×844 窄屏；标题正确、内容非空、无框架遮罩、控制台 error / pageerror 均为 0；窄屏 bodyWidth=390，无水平溢出。
- 将 ImageGen 参考与浏览器截图并排检查：白色画布、冷灰导航、青绿主色、分隔式右栏、日期/任务/倒计时层级均保持。为覆盖 00–24 点和日期导航增加滚动与导航行；短日程改用单行标题+时间，修复半小时格中文字裁切。预览提示只出现在浏览器。未把概念图样例数据写入真实 SQLite。
- 原生：实际创建 SQLite 日程；右侧展开/收起；切换左侧；侧栏勾选后进度变为 1/1；重新启动主窗仍为 1/1；系统通知测试返回成功。记录中 `fullscreen` 的 scheduled_at 和 delivered_at 都是 1793888760，证明预定时刻执行到原生展示调用。
- 诚实限制：没有完整观察到全屏倒计时、完成/暂缓/Escape 三种操作的原生呈现；没有实际令设备睡眠或改系统时区；这部分不能据 API 返回成功视为真机验收通过。还需检查多屏、Mac Spaces、系统勿扰、Windows 托盘和声音。
- 最终 macOS 发行应用约 13 MiB。初次 DMG Finder 美化流程在卸载临时磁盘时失败，已增加 `scripts/package-macos.sh` 的无 Finder 打包路径；清理了本任务遗留的临时挂载。
- 发现默认链接器签名不能通过完整 bundle 校验，已显式配置 macOS 免费 ad-hoc 签名；`codesign --verify --deep --strict` 通过。无 Apple 开发者身份签名或公证，不能承诺没有系统安全提示。
- Windows 与 Intel Mac 安装包目前只有已编写的 Actions 工作流，尚未在 GitHub 执行。待仓库创建和上传后再记录真实远端结果。

## 待验收

- [ ] macOS 全屏：可见倒计时；完成、暂缓 5 分钟、关闭、Escape；主窗同步；多条提醒排队。
- [ ] 实际睡眠恢复、系统时钟前/后调、时区改变、系统勿扰、禁用通知后的提示。
- [ ] Windows 安装、卸载、托盘、左右侧栏、多显示器缩放、通知和全屏。
- [ ] Intel Mac 真机运行；GitHub Release 三个平台资产齐全。
- [ ] 提醒声音目前只有系统默认/静音，更多铃声选择未实现。
