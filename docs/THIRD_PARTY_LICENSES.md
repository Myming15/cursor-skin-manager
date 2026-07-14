# 第三方依赖许可证

Cursor Skin Manager 自身使用 [MIT License](../LICENSE)。项目构建、测试和运行所依赖的第三方软件仍由各自作者按照其许可证提供。

锁定依赖的声明许可证清单：

- [npm 依赖清单](licenses/npm.md)
- [Cargo 依赖清单](licenses/cargo.md)

清单由 `scripts/generate-third-party-licenses.mjs` 从 `package-lock.json`、已安装的 npm 包元数据、`src-tauri/Cargo.lock` 和 Cargo 元数据生成。许可证字段来自上游包声明，仅用于维护和发布检查，不构成法律意见，也不能替代上游完整许可证文本。

更新依赖后执行：

```powershell
npm ci
npm run licenses:generate
npm run licenses:check
```

npm 清单只接受经过项目审查的许可证表达式；出现新的表达式会中止生成，维护者必须先确认其兼容性。Rust 依赖同时由 `cargo-deny` 检查已知漏洞、许可证和来源。

本清单只覆盖软件依赖，不声明导入皮肤、截图、示例素材或其他第三方内容的版权状态。完整许可证文本可在安装后的依赖目录、Cargo 缓存或对应上游项目中查看。
