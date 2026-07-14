# 简化社区发布验收

更新时间：2026-07-14

本验收按当前单维护者、Windows x64 MVP 的实际范围执行。目标是确认访客能够理解和下载项目、贡献者能够运行检查、维护者能够安全发布、最终用户能够校验产物。完整社区标签体系、代码签名和长期运营节奏不在本轮范围内。

验收结论：**通过（精简范围）**。

## 验收范围

| 视角 | 检查项 | 状态 |
| --- | --- | --- |
| 访客 | README 能说明用途、许可、下载、帮助和安全报告入口 | 已通过 |
| 贡献者 | 干净克隆后有安装、测试和 PR 指南，必需 CI 会在每个 PR 运行 | 已通过 |
| 维护者 | 主分支、发布标签、依赖告警和私密漏洞报告有受控流程 | 已通过 |
| 维护者 | Windows Release Dry Run 能生成私有 Artifact，且不创建公开 Release | 已通过 |
| 最终用户 | 安装版、便携版和 SHA-256 名称一致，校验文件可验证两个 EXE | 已通过 |
| 仓库 | 不跟踪密钥、构建目录、日志、个人开发目录或 Release 二进制 | 已通过 |

## 自动化证据

- Windows CI、Dependency Security 和 CodeQL 已作为 `main` 的必需检查运行。
- `main` Direct Push 与 Force Push 已实测被 Ruleset 拒绝。
- `v*` 标签创建受到限制，创建后的更新和删除已实测被 Ruleset 拒绝。
- 最终 Dry Run：[Windows Release #2](https://github.com/Myming15/cursor-skin-manager/actions/runs/29316049265)，提交 `4a114d24750b1361f655d6f3d83cd0c565b00f7c`。
- `Release / Verify and package` 成功；`Release / Verify uploaded artifact` 成功；`Release / Create draft` 按手动 Dry Run 规则跳过。
- 私有 Artifact ID 为 `8304098786`，名称为 `cursor-skin-manager-0.1.12-windows-x64`，大小为 `220412136` 字节，归档摘要为 `sha256:3d7f38b6ebe2ac98ceac3a6fb9936059ec9fcfee1cc7514c18281b93e2d9814f`，保留到 2026-07-28。
- 云端验证任务重新下载上传后的 Artifact，确认其中只有安装版、便携版、SHA-256 清单和 `release-notes.md`，并重新计算两个 EXE 的 SHA-256；任何文件缺失、额外文件或哈希不一致都会使任务失败。
- 本地真实 Release 构建也完成独立验证：便携版 `11467776` 字节，SHA-256 为 `31847657b37a05271f03ee0a6967964f4d652752bed6a28ebe6232a9ad096339`；安装版 `209030322` 字节，SHA-256 为 `6b92df9cd17cb89e09c71ab12812b72c8731acb89ab75b6a04a984d4ceb20ad2`。两者产品版本均为 `0.1.12`。
- Dry Run 后仓库仍只有公开 Release `v0.1.11`，没有 `v0.1.12` 标签、Draft 或公开 Release。
- 仓库当前 76 个跟踪文件中没有 `node_modules`、`dist`、`target`、`release`、密钥文件或超过 5 MB 的文件。

## 已知风险和延期项

- Windows 安装包尚未代码签名，SmartScreen 可能显示未知发布者。
- 当前只构建和验证 Windows x64，ARM64 尚未完整测试。
- 便携版依赖目标电脑已有 Microsoft Edge WebView2 Runtime。
- 每个公开版本仍需要维护者在非开发 Windows 环境完成一次 GUI 冒烟测试。
- Issue 标签和问题分流规则等到社区反馈增加后再建立。
- 第三方皮肤素材版权梳理按项目决策延期，不属于本轮验收。

## 发布公告草案

> Cursor Skin Manager 新版本现已发布。它是一款本地 Windows 光标皮肤管理工具，支持导入、预览、应用、替换和恢复默认光标。请只从本项目 GitHub Releases 下载，并使用随附的 SHA-256 文件校验安装版或便携版。当前安装包尚未代码签名，Windows 可能显示 SmartScreen 提示；遇到问题请通过 GitHub Issues 反馈，并在提交日志前移除个人路径。

## 完成条件

运行链接、Artifact 信息、SHA-256 验证和“未创建 Release”的结果均已记录。本轮简化社区发布验收完成；每个未来公开版本仍须按 `RELEASING.md` 在非开发 Windows 环境执行 GUI 冒烟测试。
