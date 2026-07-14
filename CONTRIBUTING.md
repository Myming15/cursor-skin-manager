# 参与贡献

感谢你愿意帮助改进 Cursor Skin Manager。本项目接受外部 Issue 和 Pull Request，社区交流以中文为主；使用英文提交也没有问题。

提交前请先阅读本指南。项目目前专注于 Windows 本地光标皮肤管理 MVP，贡献应保持范围清晰、可以验证，并避免破坏用户的光标配置和本地文件。

参与社区交流时还需要遵守 [社区行为准则](CODE_OF_CONDUCT.md)。不确定问题应提交到哪里时，请先查看 [支持指南](SUPPORT.md)。

## 可以贡献什么

欢迎以下类型的贡献：

- 修复导入、解析、预览、应用、恢复默认、删除和托盘功能中的 Bug。
- 改进 `.cur`、`.ani`、`install.inf`、中文文件名、GBK 配置和特殊路径的兼容性。
- 改进角色替换、未分配文件交换、应用状态检测和文件事务安全。
- 改善性能、稳定性、可访问性、错误提示、测试、文档和 Windows 打包流程。
- 提供能够稳定复现问题的最小测试样本，但你必须拥有提交和再分发这些文件的权利。

当前不接受以下方向的直接实现：

- 在线皮肤市场、账号系统、收藏、标签、社交或云同步。
- macOS、Linux、全用户系统级安装或与 Windows 光标管理无关的大型功能。
- 未先讨论的大规模重构、技术栈替换或视觉体系重做。

对于范围外但确有价值的建议，请先创建 Feature Request，说明使用场景和 MVP 影响，等待维护者确认后再开始开发。

## 报告问题

提交前请先搜索 [GitHub Issues](../../issues)，避免重复报告。

### Bug 报告

请尽量提供：

- Cursor Skin Manager 版本。
- Windows 版本、系统架构，以及安装版或便携版。
- 可稳定复现的操作步骤、预期结果和实际结果。
- 错误提示、必要的界面截图，以及 `app.log` 或 `startup.log` 中相关片段。
- 问题涉及的文件格式、数量、INF 编码和目录结构。

提交日志或截图前，请遮盖用户名、个人目录、下载地址和其他隐私信息。不要上传完整的应用数据目录、`library.json`、私人光标包或无权公开的文件。

### 功能建议

请说明要解决的问题、典型操作流程、为什么现有功能不足，以及该建议是否仍属于本地 Windows 光标管理 MVP。不要只提交界面草图或功能名称。

### 兼容性问题

请说明 `.cur`、`.ani` 或 `install.inf` 的来源类型、编码、目录结构和当前识别结果。只有在你拥有再分发权利时才可附加样本；否则请提供自行制作的最小复现文件或结构说明。

### 安全漏洞

不要为安全漏洞创建公开 Issue。请阅读 [安全策略](SECURITY.md)，并从仓库 **Security** 页面的 **Report a vulnerability** 入口通过 GitHub Private Vulnerability Reporting 私密提交。当前项目不通过公开个人邮箱接收漏洞报告。

## 开发环境

本项目是 Windows 桌面应用，完整开发和手动验证需要 Windows 10 或 Windows 11。

完整安装步骤、命令和故障排查见 [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md)。开始修改核心数据流前，请同时阅读 [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) 和 [`docs/DATA_FORMAT.md`](docs/DATA_FORMAT.md)。

请准备：

- Git。
- Node.js 24 LTS，以及随 Node.js 安装的 npm；仓库通过 `.nvmrc` 固定 CI 使用的补丁版本。
- 通过 rustup 安装的 Rust stable MSVC 工具链。
- Microsoft C++ Build Tools、Windows SDK 和 Microsoft Edge WebView2 Runtime。

仓库使用 `package-lock.json`，请使用 npm，不要提交由其他包管理器生成的锁文件。

Fork 仓库后执行：

```powershell
git clone https://github.com/YOUR_NAME/cursor-skin-manager.git
cd cursor-skin-manager
git remote add upstream https://github.com/Myming15/cursor-skin-manager.git
npm ci
npm run tauri -- dev
```

若 Vite 或 Vitest 报错 `crypto.getRandomValues is not a function`，通常是 Node.js 版本过旧。请先执行 `node --version`，并升级到受支持版本。

## 目录说明

| 路径 | 职责 |
| --- | --- |
| `src/` | React、TypeScript、界面样式和前端测试 |
| `src-tauri/src/lib.rs` | Tauri command、光标解析、文件事务、注册表和 Rust 测试 |
| `src-tauri/tauri.conf.json` | Tauri 窗口、版本、图标和打包配置 |
| `public/`、`src-tauri/icons/` | 应用图标与静态资源 |
| `scripts/` | 打包、便携版和发布辅助脚本 |
| `docs/` | 产品、品牌、版本和维护文档 |

模块调用边界、导入与应用流程见 [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)；`library.json`、备份和 15 个角色字段见 [`docs/DATA_FORMAT.md`](docs/DATA_FORMAT.md)。

## 开发流程

1. 在 Issue 中确认问题尚未被处理；较大功能应先获得维护者确认。
2. 从最新 `upstream/main` 创建单一用途分支。
3. 完成最小范围修改，并为行为变化增加或更新测试。
4. 执行提交前检查和必要的 Windows 手动验证。
5. 向 `Myming15/cursor-skin-manager` 的 `main` 分支提交 Pull Request。

同步并创建分支的示例：

```powershell
git fetch upstream
git switch main
git rebase upstream/main
git switch -c fix/short-description
```

推荐使用以下分支前缀：`fix/`、`feat/`、`docs/`、`test/`、`refactor/`、`chore/`。

提交信息应简洁说明意图，推荐使用 `fix:`、`feat:`、`docs:`、`test:`、`refactor:` 或 `chore:` 前缀。一个提交不要混入无关格式化、生成文件或版本号修改；除非维护者明确要求准备发布，否则不要自行提升应用版本。

## 实现要求

### React 与 TypeScript

- 沿用现有组件、类型、状态管理和 Tauri `invoke` 调用方式。
- 所有异步操作都要处理忙碌、成功和失败状态，并阻止重复触发。
- 新交互需要支持键盘、`focus-visible`、disabled 状态和明确的中文错误信息。
- 卡片 hover、按钮和弹窗不能造成布局跳动，也不能因为事件冒泡触发两次操作。
- 行为变化应在 `src/main.test.tsx` 或相邻测试文件中覆盖。

### Rust 与 Tauri command

- 文件系统、`library.json`、预览生成和 Windows 注册表修改必须由 Rust 后端完成，前端不得直接写入应用数据。
- 新增 command 时应使用清晰的输入输出类型、注册到 `tauri::generate_handler!`，并为核心逻辑增加 Rust 测试。
- 不信任前端传入的皮肤 ID、角色、文件名或路径；执行操作前必须重新验证。
- 错误应保留可诊断原因，但不得在普通界面提示中泄露不必要的用户数据。

### Windows 注册表与应用状态

- 只修改 `HKEY_CURRENT_USER` 范围内的光标配置，不引入管理员权限。
- 保持现有 15 个 Windows 光标角色定义和“全部已设置角色都匹配才算已应用”的规则。
- 修改正在使用的皮肤映射时，不得静默改写注册表；应将状态更新为未应用，并提示用户重新应用。
- 涉及应用、恢复默认、删除当前皮肤或刷新系统光标的修改必须进行 Windows 手动验证。

### 文件与数据事务

- 外部光标文件只能复制到应用内部目录，不得直接引用、移动或删除用户原始文件。
- 更新 `library.json` 时必须沿用原子写入与备份机制。
- 验证、复制、预览和映射更新应保持事务性；失败时回滚新文件和内存映射。
- 同名文件不得覆盖，多个角色共享文件时不得错误移动或删除。
- 必须考虑中文、空格、特殊字符路径、大小写扩展名和 GBK 来源配置。

### 界面与产品边界

- 保持现有浅色、紧凑的 Windows 工具风格和现有主色。
- 使用现有 Lucide 图标和正式弹窗，不使用浏览器原生 `confirm` 代替产品交互。
- 不添加在线市场、账号、收藏、标签等非 MVP 导航或占位功能。
- 用户可见变化应在常用窗口尺寸下检查遮挡、滚动、文本溢出和焦点顺序。

## 提交前检查

首次检出或依赖变化后执行：

```powershell
npm ci
```

每个 Pull Request 至少执行：

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

需要修复前端格式时执行 `npm run format`，再重新运行 `npm run format:check`。这些命令与 Windows CI 完全一致；Lint、格式或 Clippy 警告都会阻止 Pull Request 合并。

每个 Pull Request 都必须通过以下 5 个 GitHub 必需检查后才能合并：

- `Frontend / Test and build`
- `Rust / Format, test, and check`
- `Dependencies / npm audit and licenses`
- `Dependencies / Cargo advisories and licenses`
- `CodeQL / JavaScript and TypeScript`

`main` 不接受直接推送或 Force Push，所有代码和文档修改都通过 Pull Request。当前单维护者阶段不强制他人审批，但所有审查对话必须先解决。

修改 `package.json`、`package-lock.json`、`Cargo.toml`、`Cargo.lock`、GitHub Actions 或许可证策略时，还必须执行：

```powershell
npm run security:audit
npm run licenses:generate
npm run licenses:check
cargo deny --manifest-path src-tauri/Cargo.toml --all-features check advisories licenses sources
```

依赖 PR 必须提交同步更新的锁文件和 `docs/licenses/` 清单，说明安全或兼容性影响，并通过 Dependency Security 与 CodeQL。不要自动合并 Dependabot PR，也不要为了通过检查而静默忽略公告或放宽许可证策略；本地安装 `cargo-deny` 和审计故障排查见开发文档。

根据修改范围补充手动验证，并在 PR 中写明结果。高风险路径至少包括：

- 导入文件夹、ZIP、单个 `.cur/.ani` 和 `install.inf`。
- 静态与动态光标预览，以及中文或特殊字符路径。
- 应用皮肤、重新加载、恢复默认和“正在使用”状态检测。
- 替换角色、分配未识别文件、文件交换和失败回滚。
- 删除当前皮肤、清空全部皮肤、托盘退出和单实例启动。

只需测试受影响的手动路径，但注册表、文件删除、数据迁移或发布脚本变更应扩大验证范围。

## Pull Request 要求

PR 描述应包括：

- 问题背景和修改范围。
- 关联 Issue，例如 `Closes #123`；没有 Issue 时说明原因。
- 自动测试命令及结果。
- 已执行的 Windows 手动验证。
- UI 修改的前后截图或短视频。
- 注册表、文件事务、数据兼容性和回滚风险。

保持 PR 小而专一。维护者可能要求补充测试、缩小范围、调整交互或拆分提交。优先使用 squash 方式合并；最终合并方式和发布时间由维护者决定。审查开始后如需重写已推送历史，请先在 PR 中说明。

## 不要提交

- 未授权或来源不明的光标包、图片、字体、音频和其他素材。
- API Token、密码、证书、签名密钥、`.env` 文件或个人邮箱凭据。
- `%APPDATA%\CursorSkinManager` 中的用户数据、日志、数据库或真实下载目录。
- `node_modules/`、`dist/`、`src-tauri/target/`、`release/`、安装包和便携版等构建产物。
- 与当前 PR 无关的格式化、依赖升级、版本号或锁文件变化。

提交贡献即表示你同意按照项目的 [MIT License](LICENSE) 提供相关代码和文档，并确认你有权提交其中包含的内容。
