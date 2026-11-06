# Daymark

免费、开源、离线优先的 macOS / Windows 日历与 Todo 桌面应用。Tauri 2 + React + TypeScript + SQLite。Apache-2.0。

开发中，当前开发版本 **0.1.1**（验收中，发布保留为草稿）。以 [PRD](docs/PRD.md) 和 [验证记录](docs/VALIDATION.md) 为准，不将尚未完成的跨平台验收当成发布承诺。

## 下载安装

如果只是安装使用，不需要安装 Node.js、Rust 或下载源码。

目前公开的旧版是 [0.1.0 安装包下载页](https://github.com/cwl128/daymark/releases/tag/v0.1.0)：

- Apple 芯片 Mac：选择 `aarch64.dmg`。
- Intel Mac：选择 `x64.dmg`。
- Windows 64 位：选择 `x64-setup.exe`。

这是 **0.1.0 开发预览版**。全屏操作、睡眠恢复、多显示器等仍有待真机验收，具体状态见验证记录。下载页只应将完成构建的资产列为可用；源码仓库本身不是在线日历网站。

## 0.1.1 修复

- 侧栏启用与展开分离；收起为原生 8×56 逻辑像素窗口，展开 280 像素宽、最高 420 像素。悬停展开、离开或失焦收起，支持固定展开和左右边缘。
- 设置可选跟随系统／浅色／深色、自选强调色及两种背景色。所有窗口即时同步，旧设置与日程保留。
- 三步使用引导可跳过、重新打开；升级后显示一次，不生成示例日程。
- 原生通知权限检查与主动授权、后台 5 秒自测、结果确认及系统设置入口。发送接口接受仅显示“已交给系统”，不视为用户已收到。

0.1.1 尚未通过完整真机验收，不覆盖旧版公开下载。验收状态见 [验证记录](docs/VALIDATION.md)。

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
