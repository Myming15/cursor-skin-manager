# 开发环境

本文说明如何在一台干净的 Windows 电脑上运行、测试和构建 Cursor Skin Manager。贡献范围与 Pull Request 规则请先阅读 [`CONTRIBUTING.md`](../CONTRIBUTING.md)，系统边界与数据流见 [`ARCHITECTURE.md`](ARCHITECTURE.md)，本地 JSON 和目录字段见 [`DATA_FORMAT.md`](DATA_FORMAT.md)。

## 支持的开发平台

完整桌面功能只在 Windows 上运行，因为后端会访问 Windows 当前用户注册表、系统光标刷新 API、系统托盘和开机自启。

- 推荐 Windows 10 或 Windows 11 64 位。
- 当前主要验证目标是 `x86_64-pc-windows-msvc`。
- ARM64 可以作为兼容性贡献目标，但尚未完成与 x64 相同范围的手动验收。
- 浏览器模式可以预览 React 界面，但不能代替 Windows Tauri 桌面验证。

## 推荐工具链

| 工具 | 推荐版本 | 说明 |
| --- | --- | --- |
| Git | 当前受支持版本 | 用于克隆、分支和提交 |
| Node.js | **24.14.0 LTS** | `.nvmrc` 与 Windows CI 固定使用的版本 |
| npm | 随 Node.js LTS 安装 | 仓库使用 `package-lock.json`，不要混用其他锁文件 |
| Rust | stable MSVC | 使用 rustup 管理；本项目已使用 `rustc 1.96.1` 验证 |
| Tauri CLI | 仓库本地版本 | `package-lock.json` 当前锁定 `tauri-cli 2.11.4`，不要求全局安装 |
| Visual Studio Build Tools | Visual Studio 2022 Build Tools 17.x 最新维护版 | 安装 **Desktop development with C++** 和 Windows 10/11 SDK |
| WebView2 | Evergreen Runtime | Tauri Windows WebView 必需；Windows 11 通常已预装 |

Node.js 的支持状态会变化。当前开发与 CI 固定为 `.nvmrc` 中的 `24.14.0`；升级前应核对 [Node.js 官方发布页](https://nodejs.org/en/about/previous-releases)，并在独立 PR 中同步本文和 CI。Windows 原生依赖安装以 [Tauri 2 Windows prerequisites](https://v2.tauri.app/start/prerequisites/) 为准。

### Visual Studio Build Tools

打开 Visual Studio Installer，安装或修改 **Build Tools for Visual Studio 2022**，至少选择：

- Desktop development with C++。
- MSVC v143 x64/x86 build tools。
- Windows 10 SDK 或 Windows 11 SDK。

`tauri.conf.json` 的 bundle target 为 `all`。如果构建 MSI 时出现 `failed to run light.exe`，检查 Windows 可选功能中的 **VBSCRIPT** 是否启用；只运行开发窗口和 Rust 测试时通常不需要该功能。

### Rust MSVC 工具链

通过 rustup 安装 Rust，并确保默认工具链是 MSVC：

```powershell
rustup default stable-msvc
rustup show active-toolchain
```

x64 开发环境应显示类似 `stable-x86_64-pc-windows-msvc`。修改工具链后请重新打开 PowerShell 和编辑器。

## 获取项目

只阅读源码可以直接克隆主仓库；提交贡献时请先 Fork，并按照贡献指南配置 `upstream`。

```powershell
git clone https://github.com/Myming15/cursor-skin-manager.git
cd cursor-skin-manager
npm ci
```

`npm ci` 严格使用 `package-lock.json`，并安装项目本地 Tauri CLI。首次启动 Tauri 时，Cargo 还会下载和编译 Rust 依赖，耗时通常高于后续启动。

使用 nvm-windows 等版本管理器时，可以先按照 `.nvmrc` 切换 Node：

```powershell
nvm install 24.14.0
nvm use 24.14.0
```

不使用版本管理器时，直接安装相同版本的 Node.js 24 LTS 即可。

安装后核对环境：

```powershell
node --version
npm --version
rustc --version
cargo --version
rustup show active-toolchain
npm run tauri -- --version
```

预期 Node 主版本为 `24`，Tauri CLI 当前为 `2.11.4`，Rust 工具链名称以 `-pc-windows-msvc` 结尾。补丁版本可以高于本文记录值。

## 运行方式

### 浏览器界面模式

```powershell
npm run dev
```

Vite 固定监听 `http://localhost:1420`。该模式使用 `demoSkins` 展示静态界面，以下能力不可用：

- Tauri 文件选择器。
- 真实本地皮肤库。
- 文件复制、预览解析和日志。
- Windows 注册表、光标刷新、托盘和开机自启。

浏览器模式适合快速检查布局和纯 React 交互，不能用于验收导入、应用或文件事务。

### Tauri 桌面模式

```powershell
npm run tauri -- dev
```

该命令先启动 Vite，再编译 Rust 后端并打开原生窗口。桌面模式会使用当前 Windows 用户的真实数据目录和注册表；测试应用、恢复默认和删除流程前，请确认当前光标配置可以恢复。

开发进程依附于启动它的 PowerShell。关闭终端会结束开发应用，这是开发模式的正常行为；正式 Release 使用 Windows GUI subsystem，不会附带命令窗口。

### 前端生产预览

```powershell
npm run build
npm run preview
```

该模式只预览 `dist/` 中的前端产物，仍不提供 Tauri 原生功能。

## 测试与静态检查

提交 Pull Request 前执行：

```powershell
npm run lint
npm run format:check
npm run test:ui
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```

前端使用固定版本的 ESLint 与 Prettier。`npm run lint` 检查 TypeScript、React Hooks 和 Vite Fast Refresh 规则；`npm run format:check` 只检查格式，`npm run format` 才会写入统一格式。受 Prettier 管理的文本类型通过 `.gitattributes` 固定为 LF，避免 Windows 和其他开发环境产生不同结果。Rust Clippy 使用 `-D warnings`，任何新警告都按错误处理。

当前测试范围：

- Vitest + Testing Library：按钮触发边界、忙碌状态、反馈自动消失、角色分配弹窗、键盘焦点和 hover 样式。
- Rust：CUR/DIB 与 ANI 预览、GBK INF、备份写入、替换/分配事务、共享文件、同名文件、回滚和应用状态。

Rust 测试默认使用自行生成的临时样本。需要额外验证真实光标时，可以临时设置：

```powershell
$env:CSM_REAL_CUR = "C:\path\to\sample.cur"
$env:CSM_REAL_ANI = "C:\path\to\sample.ani"
cargo test --manifest-path src-tauri/Cargo.toml validates_real_cursor_samples_when_configured
Remove-Item Env:\CSM_REAL_CUR -ErrorAction SilentlyContinue
Remove-Item Env:\CSM_REAL_ANI -ErrorAction SilentlyContinue
```

只使用你有权测试的文件，不要把私人或第三方光标包加入仓库。

## 依赖与供应链检查

GitHub 的 Dependency Security 工作流会审计 npm 与 Cargo 依赖，CodeQL 会扫描 JavaScript 和 TypeScript。修改依赖、锁文件、GitHub Actions 或许可证策略时，应在本地额外执行：

```powershell
npm run security:audit
npm run licenses:generate
npm run licenses:check
cargo deny --manifest-path src-tauri/Cargo.toml --all-features check advisories licenses sources
```

`security:audit` 显式使用 npm 官方注册表，避免本机镜像没有实现 audit 接口而产生错误结果。高危和严重 npm 漏洞会使命令失败；不要使用 `--force` 自动接受破坏性升级，应选择最小兼容修复并重新运行完整测试。

本地没有 `cargo-deny` 时安装仓库当前使用的版本：

```powershell
cargo install cargo-deny --locked --version 0.20.2
```

`cargo-deny` 会检查实际发布的 `x86_64-pc-windows-msvc` 依赖图中的已知公告、允许的许可证和 crates.io 来源，并将该依赖图内的 unsound 公告全部视为错误。`Cargo.lock` 仍可能包含 Tauri 为其他平台解析的依赖；处置仅存在于未发布目标的告警时，必须先用 `cargo tree --target <目标> -i <crate>` 证明目标范围，并按照 [`SECURITY.md`](../SECURITY.md#依赖与供应链安全) 记录复查条件。

`npm run licenses:generate` 会根据当前锁文件和已安装包更新 `docs/licenses/npm.md` 与 `docs/licenses/cargo.md`。生成前先运行 `npm ci`，提交时同时审查依赖版本和许可证变化，不要手工编辑生成清单。完整说明见 [`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md)。

## 仓库保护与紧急处理

`main` 只接受 Pull Request，并要求前端、Rust、npm 依赖、Cargo 依赖和 CodeQL 共 5 个检查通过。Direct Push、Force Push 和分支删除均被阻止；当前单维护者阶段不要求批准数量，但所有审查对话必须解决。

仓库所有者 `Myming15` 只有在 Pull Request 内才能使用紧急绕过，不能借此直接推送到 `main`。只有等待检查会扩大安全事故、发布中断或数据损坏风险时才可使用：

1. 创建最小范围的紧急 PR，在描述中记录事件、绕过原因、已经完成的验证和待补检查。
2. 只在 GitHub 合并界面显式选择绕过，不通过本地 Direct Push 绕过审查轨迹。
3. 风险解除后立即补跑缺失检查；发现问题时使用新的修复 PR，不重写 `main` 历史。
4. 在原 PR 中补充最终结果和后续行动，保留可审计记录。

`v*` Release 标签只允许仓库所有者创建，并由独立 Ruleset 禁止更新、Force Push 和删除。发布错误应递增版本重新发布，不能移动或复用旧标签。只有标签包含泄露凭据、违法内容或其他必须立即下架的高风险内容时，才允许按以下方式临时处理：

1. 先撤销凭据、下架 Release 资产并记录受影响标签和原因。
2. 在 GitHub `Settings > Rules > Rulesets` 临时禁用 `Immutable release tags`，只处理受影响标签。
3. 处理完成后立即重新启用 Ruleset，检查规则历史，并使用新的版本号发布修复。

不得为了修正普通打包错误而删除或移动公开标签。

## 生产构建

```powershell
npm run tauri -- build
```

Tauri 会先执行 `npm run build`，再编译 Rust Release 并在 `src-tauri/target/release/bundle/` 下生成当前配置允许的 Windows bundle。安装版使用离线 WebView2，因此体积明显大于便携程序。

本地产物用于验证时不要提交到 Git；`dist/`、`src-tauri/target/` 和 `release/` 已被忽略。只有维护者准备正式发布时才同步版本、校验安装包和生成 SHA-256，具体流程会在发布文档中维护。

本步骤只记录生产命令，不要求每个文档 PR 生成应用二进制。可以用以下命令检查 CLI 参数而不执行构建：

```powershell
npm run tauri -- build --help
```

## 应用数据与日志

开发版和正式版默认共用当前用户数据目录：

```text
%APPDATA%\CursorSkinManager
```

| 路径 | 内容 |
| --- | --- |
| `library.json` | 皮肤列表、角色映射和内部文件路径 |
| `library.bak.json` | 上一次成功替换前的库备份 |
| `settings.json` | 当前仅保存 `closeToTray` |
| `settings.bak.json` | 设置备份 |
| `skins\<skin-id>\` | 导入后的应用内部副本、替换文件和预览缓存 |
| `app.log` | command 开始、成功、失败及维护操作 |
| `startup.log` | 进程启动、Tauri 初始化、托盘降级和 panic 诊断 |

字段和恢复规则见 [`DATA_FORMAT.md`](DATA_FORMAT.md)。日志可能包含本地路径，提交 Issue 前必须脱敏。

## 清理测试数据

优先使用应用设置中的“恢复默认光标”和“清空全部皮肤”。这会按应用现有逻辑处理正在使用的皮肤，并保留设置与日志供排查。

需要完全隔离数据时：

1. 先恢复 Windows 默认光标。
2. 从托盘菜单选择“退出”，确认应用进程已经结束。
3. 将数据目录重命名为带时间戳的备份，不直接删除。

```powershell
$dataDir = [IO.Path]::GetFullPath((Join-Path $env:APPDATA "CursorSkinManager"))
$expected = [IO.Path]::GetFullPath((Join-Path $env:APPDATA "CursorSkinManager"))
if ($dataDir -ne $expected) { throw "Unexpected data directory: $dataDir" }

if (Test-Path -LiteralPath $dataDir) {
  $backup = "$dataDir.dev-backup-$(Get-Date -Format 'yyyyMMdd-HHmmss')"
  Move-Item -LiteralPath $dataDir -Destination $backup
  Write-Host "Moved test data to $backup"
}
```

下次启动会创建空数据目录。需要恢复时先退出应用，再把备份目录移回原名。不要在应用仍运行时移动 `library.json` 或皮肤目录。

## 常见开发问题

### `crypto.getRandomValues is not a function`

这是旧版 Node.js 启动 Vite/Vitest 时的典型错误。检查实际命令来源：

```powershell
node --version
where.exe node
Get-Command node -All
```

如果仍显示 Node 16、18 或已经停止支持的 Node 版本，请安装 Node.js 24 LTS，关闭所有终端后重新打开，再执行 `npm ci`。仅修改 PATH 但继续使用旧终端通常不会生效。

### 端口 1420 已被占用

Vite 配置启用了 `strictPort`，不会自动换端口。先找出占用进程：

```powershell
Get-NetTCPConnection -LocalPort 1420 -ErrorAction SilentlyContinue |
  Select-Object LocalAddress, LocalPort, State, OwningProcess
```

确认进程属于你自己的旧开发会话后再手动结束，避免停止无关程序。

### 找不到 `link.exe` 或 Windows SDK

重新打开 Visual Studio Installer，确认已安装 Desktop development with C++、MSVC v143 和 Windows SDK。安装后重启终端；必要时从 “x64 Native Tools Command Prompt for VS 2022” 验证。

### WebView2 窗口无法打开

安装 Microsoft Edge WebView2 Evergreen Runtime。便携版依赖系统已有 Runtime；正式安装包配置为离线安装 WebView2。

### 关闭窗口后进程仍存在

默认设置是关闭到托盘。开发结束时从托盘菜单选择“退出”。单实例插件会让再次启动只显示现有主窗口，而不是创建第二个后台实例。
