## 修改说明 / Summary

<!-- 说明问题背景、这次修改做了什么，以及明确没有做什么。 -->

## 关联问题 / Related issue

<!-- 使用 Closes #123 或说明为什么没有对应 Issue。 -->

Closes #

## 修改类型 / Change type

- [ ] Bug 修复 / Bug fix
- [ ] 功能改进 / Feature improvement
- [ ] 光标兼容性 / Cursor compatibility
- [ ] 重构或性能 / Refactor or performance
- [ ] 测试 / Tests
- [ ] 文档或社区 / Documentation or community
- [ ] 构建或发布 / Build or release

## 行为与影响范围 / Behavior and scope

<!-- 描述用户可见变化、受影响模块，以及未受影响的现有流程。 -->

## 自动检查 / Automated checks

- [ ] `npm run test:ui`
- [ ] `npm run build`
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml`

<!-- 写明未执行或失败的检查及原因，不能只勾选而不说明。 -->

**测试结果 / Test results:**

## Windows 手动验证 / Windows manual verification

<!-- 列出 Windows 版本、安装方式、测试步骤和结果；纯文档修改可填写“不适用”。 -->

- 环境 / Environment:
- 验证步骤 / Steps:
- 验证结果 / Result:

## 界面截图 / UI evidence

<!-- UI 修改请提供前后截图或短视频；无 UI 变化请填写“不适用”。截图必须遮盖私人路径和个人信息。 -->

## 数据与安全影响 / Data and security impact

- [ ] 不修改注册表、应用数据或用户文件 / No registry, app-data, or user-file changes
- [ ] 修改了上述行为，并已在下方说明验证、事务和回滚 / Changes these areas and documents validation, transaction, and rollback below

<!-- 二选一。涉及文件或注册表时，说明 HKCU 范围、外部原文件保护、原子写入、备份和失败回滚。 -->

**风险与回滚 / Risks and rollback:**

## 兼容性 / Compatibility

<!-- 说明 CUR/ANI/INF、中文或特殊路径、旧 library.json、安装版/便携版等兼容性影响。 -->

## 提交清单 / Final checklist

- [ ] 修改保持单一范围，并遵循 `CONTRIBUTING.md` 与 `CODE_OF_CONDUCT.md`。
- [ ] 行为变化已增加或更新测试，必要文档也已同步。
- [ ] 没有自行修改版本号；发布版本变更仅按维护者要求进行。
- [ ] 没有提交密钥、个人数据、真实应用数据目录或构建产物。
- [ ] 没有提交未授权或来源不明的光标包及其他素材。
- [ ] 安全漏洞没有通过公开 PR 披露，已改用 Private Vulnerability Reporting。
