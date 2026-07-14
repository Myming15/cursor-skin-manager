# Windows 发布与回滚

本文供 Cursor Skin Manager 维护者使用。当前发布目标是 Windows x64，自动化负责校验、测试、打包和生成 Draft Release；维护者完成冒烟验证后才可以手动公开 Release。

当前安装包尚未接入商业代码签名。发布流程不会读取 `.pfx`、私钥或签名密码，也不会承诺消除 Windows SmartScreen 提示。

## 安全模型

- 手动运行 `Windows Release` 工作流只执行 Dry Run，上传保留 14 天的私有 Artifact，不创建标签或 GitHub Release。
- 只有仓库所有者可以创建 `v*` 标签；标签创建后禁止更新、移动和删除。
- `v*` 标签触发完整验证和 Windows 构建，成功后只创建 Draft Release，不会自动公开或设为 Latest。
- 已存在的 Release 不会被自动化覆盖，公开版本出现问题时使用更高版本号修复。

## 发布文件

版本 `X.Y.Z` 会生成：

| 文件 | 用途 |
| --- | --- |
| `cursor-skin-manager-X.Y.Z-windows-x64-setup.exe` | 包含离线 WebView2 的 NSIS 安装版 |
| `cursor-skin-manager-X.Y.Z-windows-x64-portable.exe` | 依赖目标电脑已有 WebView2 Runtime 的便携版 |
| `cursor-skin-manager-X.Y.Z-windows-x64-sha256.txt` | 安装版与便携版的 SHA-256 |

工作流还会在私有 Artifact 中保留 `release-notes.md`，其内容来自 `CHANGELOG.md`。

## 发布前准备

1. 从最新 `main` 创建发布准备分支，不直接修改主分支。
2. 同步更新以下版本来源，且必须完全一致：
   - `package.json`
   - `package-lock.json` 顶层版本和根包版本
   - `src-tauri/Cargo.toml`
   - `src-tauri/Cargo.lock` 中应用包版本
   - `src-tauri/tauri.conf.json`
3. 将 `CHANGELOG.md` 的“未发布”内容整理为 `## [X.Y.Z] - YYYY-MM-DD`，并补充版本比较链接。
4. 执行本地检查：

```powershell
npm ci
npm run release:validate
npm run lint
npm run format:check
npm run test:ui
npm run test:release
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```

5. 通过 Pull Request 合并，并确认 `main` 的 5 项必需检查全部成功。

## Dry Run

每次创建正式标签前，都要对准备发布的同一 `main` 提交执行一次 Dry Run：

1. 打开 GitHub Actions 的 `Windows Release`。
2. 点击 `Run workflow`，选择 `main` 后确认。
3. 等待 `Release / Verify and package` 完成；手动运行不会执行 `Release / Create draft`。
4. 下载名称为 `cursor-skin-manager-X.Y.Z-windows-x64` 的 Artifact。
5. 确认包含两个 EXE、SHA-256 文件和 `release-notes.md`，且没有 Draft 或公开 Release 被创建。

在解压目录中验证 SHA-256：

```powershell
$checksumFile = Get-ChildItem -Filter '*-sha256.txt' | Select-Object -First 1
$expected = @{}
Get-Content -LiteralPath $checksumFile.FullName | ForEach-Object {
  if ($_ -match '^([0-9a-fA-F]{64})\s{2}(.+)$') {
    $expected[$matches[2]] = $matches[1].ToLowerInvariant()
  }
}

foreach ($name in $expected.Keys) {
  $actual = (Get-FileHash -LiteralPath $name -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -ne $expected[$name]) { throw "SHA-256 mismatch: $name" }
}
```

## 冒烟测试

公开前至少检查：

- 安装版可以安装、启动和卸载，桌面与托盘图标正常。
- 便携版在已安装 WebView2 Runtime 的电脑上可以启动。
- 重复启动只唤醒已有窗口，不产生重复托盘图标。
- 可以导入一套 CUR/ANI 皮肤、预览、应用并恢复默认光标。
- 关闭到托盘、从托盘退出、日志和数据目录入口正常。
- 安装版和便携版显示的设置版本号与 Release 一致。
- SHA-256 文件能够验证两个 EXE。

建议至少在一台不包含开发环境的 Windows 10 或 Windows 11 x64 电脑上完成安装版测试。

## 创建 Draft Release

只有 Dry Run 和冒烟测试通过后才创建标签：

```powershell
git switch main
git pull --ff-only origin main
git tag -a vX.Y.Z -m "Cursor Skin Manager vX.Y.Z"
git push origin vX.Y.Z
```

标签会触发 `Windows Release`。工作流重新执行全部检查和构建，然后创建 Draft Release。维护者需要下载 Draft 中的文件，重新核对名称、SHA-256、Release Notes 和版本号，最后在 GitHub 页面手动发布并按需设为 Latest。

不要在工作流运行期间重复推送标签，也不要移动已有标签。

## 失败和回滚

### 标签创建前失败

修复原发布准备分支，重新通过 PR 和 Dry Run。此时没有公开标签，可以继续使用计划中的版本号。

### 标签创建后、公开前失败

不要移动或删除标签。保留失败记录，修复问题并递增补丁版本，例如从 `0.1.12` 改为 `0.1.13`，重新执行完整流程。错误 Draft 可以关闭或删除，但对应标签不得复用。

### 公开后发现严重问题

1. 在 Release Notes 顶部增加醒目警告，取消 Latest 状态或将 Release 标记为预发布。
2. 如果文件会造成安全或数据风险，先停止继续分发相关资产并发布安全说明。
3. 从最新 `main` 创建修复版本，递增补丁版本并完整执行 Dry Run、标签构建和冒烟测试。
4. 不静默替换旧版本资产，不重写公开标签。

只有标签包含泄露凭据、违法内容或必须立即下架的高风险内容时，才允许按 `docs/DEVELOPMENT.md` 的紧急流程临时禁用不可变标签 Ruleset。普通打包错误不使用紧急绕过。

## 发布后检查

- `README.md` 的稳定版本和下载链接指向新 Release。
- `CHANGELOG.md` 已包含版本日期与比较链接。
- GitHub Release 保留安装版、便携版和 SHA-256 文件。
- 创建新的“未发布”章节，后续开发继续记录在其中。
- 检查 Windows CI、Dependency Security、CodeQL 和 Release 工作流均无失败。
