# Daymark

免费、开源、离线优先的 macOS / Windows 日历与 Todo 桌面应用。Tauri 2 + React + TypeScript + SQLite。Apache-2.0。

开发中，当前版本 **0.1.0**。以 [PRD](docs/PRD.md) 和 [验证记录](docs/VALIDATION.md) 为准，不将尚未完成的跨平台验收当成发布承诺。

## 本地运行

安装 Node.js 22+、pnpm 11.25.0、稳定版 Rust，以及 [Tauri 系统依赖](https://v2.tauri.app/start/prerequisites/)。

```sh
pnpm install --frozen-lockfile
pnpm tauri dev
```

```sh
pnpm test
pnpm build
cd src-tauri
cargo test --locked
```

生成当前系统安装包：`pnpm tauri build`。macOS 输出在 `src-tauri/target/release/bundle/dmg/`；Windows NSIS 输出在 `src-tauri/target/release/bundle/nsis/`。

## 使用与隐私

创建日程或任务后，在日/周/月视图浏览，勾选记录完成。悬浮侧边栏可在左/右边缘收放。关闭主窗口后，应用继续运行；从菜单栏/托盘重新打开，选择「退出」才停止提醒。

SQLite 位于系统应用数据目录下的 `org.daymark.desktop/daymark.db`（由 Tauri 解析具体目录）。应用不包含登录、遥测、广告或远程 API。所有时间以 UTC 保存，按当前系统时区显示。

系统通知需要权限；勿扰和系统声音设置可能影响提醒。仅应用运行且设备唤醒时能准点提醒；唤醒后补发最近 15 分钟内最紧迫的一档。完全退出后不运行系统服务。详细策略见 PRD。

`pnpm dev` 提供浏览器 UI 预览：使用独立 localStorage 测试数据，明确显示预览提示；不模拟原生 SQLite、系统通知或全屏窗口。

## 免费发行

推送 `v*` 标签会在 GitHub Actions 中运行检查，并构建 macOS ARM64 / Intel 和 Windows x64 安装包，保存为 GitHub Release 草稿。人工验收后再公开发布。未配置签名时构建未签名安装包，系统可能提示未知开发者。项目没有付费运行依赖，签名凭据属于可选发布配置。

macOS 的 Finder 打包流程若报临时磁盘忙，可使用不依赖 Finder 美化的备用方式：

```sh
pnpm tauri build --bundles app
bash scripts/package-macos.sh
```

macOS 使用免费 ad-hoc 签名并校验 bundle 完整性，未进行 Apple 公证。它不提供开发者身份信任背书。Windows 预览安装包也未配置商业签名。

UI 回归：`pnpm test:e2e`。本机使用 Chrome；CI 会安装 Chromium。原生提醒的人工验收清单见验证记录。
