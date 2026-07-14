# 更新日志

本文记录 Cursor Skin Manager 面向用户和贡献者的重要变化。格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/) 的 `0.x` 开发阶段规则。

## [未发布]

### 依赖与构建

- 将 `tauri-plugin-single-instance` 从 `2.4.2` 更新到 `2.4.3`。
- 更新 Testing Library 的 Jest DOM、React 和 User Event 开发依赖。
- 同步 npm、Cargo 锁文件和第三方许可证清单，源码版本递增为 `0.1.13`。

## [0.1.12] - 2026-07-14

### 文档与社区

- 采用 MIT License，并建立贡献指南、行为准则、支持指南和安全策略。
- 增加 Bug、功能建议、光标兼容性 Issue Form 和 Pull Request 模板。
- 增加开发环境、系统架构、本地数据格式、更新日志和路线图文档。
- 增加 Windows GitHub Actions，在 Push 和 Pull Request 中自动执行前端与 Rust 检查。
- 固定 ESLint、React Hooks、Prettier 工具链，并将前端 Lint、格式检查和零警告 Clippy 设为阻塞质量门禁。
- 增加 Windows Release Dry Run 与不可变版本标签触发的 Draft Release 自动化。
- 增加版本一致性、稳定资产命名、SHA-256、发布回滚和简化社区验收流程。

### 安全与依赖

- 将 Vite 升级到 `6.4.3`、Vitest 升级到 `3.2.7`，修复 npm 审计发现的已知开发工具漏洞。
- 增加 npm、Cargo 和 GitHub Actions 三类每周 Dependabot 更新，并限制同时打开的更新 PR 数量。
- 增加 npm 高危漏洞审计、Cargo 公告/许可证/来源审计，以及 JavaScript/TypeScript CodeQL 扫描。
- 将 Cargo 安全审计限定到实际发布的 Windows x64 依赖图，并阻塞该图内的全部 unsound 公告。
- 将 GitHub Actions 固定到完整提交 SHA，并保持工作流最小权限。
- 增加锁定 npm 与 Cargo 依赖的许可证清单和变更门禁。

本版本完善了社区协作、安全检查和 Windows 发布自动化，并重新生成安装版与便携版。

## [0.1.11] - 2026-07-13

首个具有可验证 Git 标签和 GitHub Release 的公开版本。

### 新增

- 支持导入文件夹、ZIP、单个 `.cur/.ani` 和包含 `install.inf` 的光标皮肤包。
- 自动识别 15 个 Windows 光标角色，预览静态 CUR 和动态 ANI 的首个可解析帧。
- 支持一键应用到 Windows 当前用户、重新加载光标和恢复 Windows 默认光标。
- 支持删除单个皮肤和清空全部皮肤；删除正在使用的皮肤前自动恢复默认光标。
- 支持替换指定角色的 CUR/ANI，以及把未分配文件交换到任意角色。
- 支持开机自启、关闭窗口时最小化到托盘、打开皮肤目录和打开日志。
- 增加单实例启动、托盘失败降级、启动日志和原生启动错误提示。
- 提供高分辨率、多尺寸、透明边缘的 Windows 应用图标。

### 兼容性

- 支持 CUR 中的 PNG 与常见 DIB 图像，以及 RIFF/ACON ANI 容器。
- INF 文本支持 UTF-8、带 BOM 的 UTF-16 和常见 GBK 来源文件。
- 支持中文文件名、空格和特殊字符路径。
- 安装版包含离线 WebView2；便携版使用目标电脑已有的 WebView2 Runtime。

### 数据安全

- 外部皮肤和替换文件只复制到应用内部目录，不修改或删除原始下载文件。
- `library.json` 和设置文件使用临时文件、备份与恢复机制写入。
- 替换和重新分配失败时回滚映射，并清理本次新增文件和预览。
- 同名文件使用无覆盖命名；多个角色共享文件时保留其他角色引用。
- 只修改 `HKEY_CURRENT_USER` 光标配置，不要求管理员权限。
- 修改正在使用的皮肤后重新检测为未应用，不静默修改 Windows 注册表。

### 修复

- 成功提示会自动消失，持续错误仍保留失败原因供用户查看。
- 角色卡片和未分配文件卡片不再整卡触发操作，只有明确按钮可以替换或分配。
- 修复重复启动产生多个托盘实例的问题，后续启动会唤醒已有主窗口。
- 修复 Release 程序显示命令窗口、托盘初始化失败导致启动中止和跨电脑启动缺少诊断的问题。
- 改善小尺寸任务栏、托盘和桌面快捷方式图标的清晰度与视觉占比。
- 统一皮肤列表预览的透明边界裁剪和居中方式。

### 分发

- 提供 Windows x64 安装版、便携版和 SHA-256 校验文件。
- 当前安装包未进行商业代码签名，Windows SmartScreen 仍可能显示未知发布者提示。

## 未单独公开发布的开发里程碑

以下版本号存在于开发记录中，但仓库没有对应的独立公开标签或 Release。相关变化最终包含在 `v0.1.11` 中，不应把这些里程碑当作可下载历史版本。

### 0.1.10

- 增加 15 个角色的单独文件替换。
- 增加未分配文件的角色选择和安全交换。
- 增加 CUR/ANI 内容验证、同名无覆盖复制、共享引用保护和失败回滚测试。

### 0.1.1

- 重做小尺寸应用图标，并生成 16、20、24、32、40、48、64、128、256px ICO 图层。

[未发布]: https://github.com/Myming15/cursor-skin-manager/compare/v0.1.12...HEAD
[0.1.12]: https://github.com/Myming15/cursor-skin-manager/compare/v0.1.11...v0.1.12
[0.1.11]: https://github.com/Myming15/cursor-skin-manager/releases/tag/v0.1.11
