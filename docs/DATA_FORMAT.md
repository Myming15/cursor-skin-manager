# 数据格式

本文描述 Cursor Skin Manager 当前使用的本地目录和 JSON 字段。它用于开发、迁移和故障排查，不表示 `library.json` 是对外稳定 API。

修改数据结构前请同时阅读 [`ARCHITECTURE.md`](ARCHITECTURE.md) 中的事务与恢复边界。

## 数据根目录

应用使用固定目录：

```text
%APPDATA%\CursorSkinManager\
├─ skins\
│  └─ <skin-id>\
│     ├─ <imported cursor files>
│     ├─ custom\
│     └─ .previews\
├─ library.json
├─ library.bak.json
├─ settings.json
├─ settings.bak.json
├─ app.log
└─ startup.log
```

| 路径 | 内容 |
| --- | --- |
| `skins/<skin-id>/` | 应用管理的皮肤副本，删除皮肤时只删除这里的内容 |
| `custom/` | 用户通过“替换文件”加入当前皮肤的复制文件 |
| `.previews/` | 由后端生成的居中 PNG 预览缓存 |
| `library.json` | 皮肤列表、角色映射和未分配文件 |
| `library.bak.json` | 最近一次有效主数据备份 |
| `settings.json` | 应用自身设置 |
| `settings.bak.json` | 最近一次有效设置备份 |
| `app.log` | 运行期操作和错误日志 |
| `startup.log` | Tauri runtime 之前的启动错误 |

`sourcePath` 指向用户选择的原始来源，`storagePath` 和所有持久化光标路径必须指向应用内部副本。应用不得移动或删除原始来源。

## `library.json`

根值是皮肤对象数组。以下示例使用占位路径，真实文件保存 Windows 绝对路径：

```json
[
  {
    "id": "example-skin-1700000000",
    "name": "Example Skin",
    "sourcePath": "<original source path>",
    "storagePath": "<app data path>\\skins\\example-skin-1700000000",
    "importedAt": "1700000000",
    "hasInf": true,
    "isComplete": false,
    "cursorCount": 3,
    "isApplied": false,
    "importNote": null,
    "roles": [
      {
        "role": "Normal Select",
        "windowsKey": "Arrow",
        "filePath": "<app data path>\\skins\\example-skin-1700000000\\Arrow.cur",
        "fileName": "Arrow.cur",
        "previewPath": "<app data path>\\skins\\example-skin-1700000000\\.previews\\Arrow.cur-<hash>.centered-v1.png",
        "previewDataUrl": null,
        "type": "CUR",
        "exists": true
      }
    ],
    "unassignedFiles": []
  }
]
```

实际皮肤始终包含全部 15 个 `roles` 条目。示例为便于阅读只展示一个角色。

### 皮肤字段

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `id` | `string` | 皮肤内部唯一 ID，同时用作默认存储目录名 |
| `name` | `string` | 界面显示名称，通常来自来源文件夹、INF 或文件名 |
| `sourcePath` | `string` | 导入时的原始来源绝对路径，只用于来源提示和重复导入判断 |
| `storagePath` | `string` | 当前皮肤应用内部目录的绝对路径 |
| `importedAt` | `string` | 当前实现保存为 Unix epoch 秒的字符串 |
| `hasInf` | `boolean` | 导入分析时是否找到并使用 INF 信息 |
| `isComplete` | `boolean` | 15 个角色的文件当前是否全部存在，由后端刷新 |
| `cursorCount` | `number` | `storagePath` 中当前扫描到的 `.cur/.ani` 文件总数 |
| `isApplied` | `boolean` | 已设置角色是否全部匹配当前用户注册表，由后端重新检测 |
| `importNote` | `string \| null` | 解析降级、不完整映射等简短诊断说明 |
| `roles` | `CursorRole[]` | 固定顺序的 15 个 Windows 光标角色 |
| `unassignedFiles` | `CursorFile[]` | 已导入但当前没有映射到角色的内部文件 |

`isComplete`、`cursorCount`、`isApplied` 和各对象的 `exists` 都属于可重新计算状态。读取库后，Rust 后端会根据磁盘和注册表刷新它们；调用方不应把旧 JSON 中的值视为最终事实。

## `CursorRole`

```ts
type CursorRole = {
  role: string;
  windowsKey: string;
  filePath: string | null;
  fileName: string | null;
  previewPath: string | null;
  previewDataUrl: string | null;
  type: "CUR" | "ANI" | null;
  exists: boolean;
};
```

未设置角色的 `filePath`、`fileName`、`previewPath` 和 `type` 可以为 `null`，`exists` 为 `false`。

### 15 个角色

角色显示名称和 Windows 注册表值名必须保持以下对应关系：

| 顺序 | `role` | `windowsKey` |
| --- | --- | --- |
| 1 | `Normal Select` | `Arrow` |
| 2 | `Help Select` | `Help` |
| 3 | `Working In Background` | `AppStarting` |
| 4 | `Busy` | `Wait` |
| 5 | `Precision Select` | `Crosshair` |
| 6 | `Text Select` | `IBeam` |
| 7 | `Handwriting` | `NWPen` |
| 8 | `Unavailable` | `No` |
| 9 | `Vertical Resize` | `SizeNS` |
| 10 | `Horizontal Resize` | `SizeWE` |
| 11 | `Diagonal Resize 1` | `SizeNWSE` |
| 12 | `Diagonal Resize 2` | `SizeNESW` |
| 13 | `Move` | `SizeAll` |
| 14 | `Alternate Select` | `UpArrow` |
| 15 | `Link Select` | `Hand` |

`windowsKey` 对应 `HKEY_CURRENT_USER\Control Panel\Cursors` 下的值名。不要仅修改前端标签来新增或重排角色；后端定义、INF 映射、注册表逻辑、测试和迁移都必须一致更新。

## `CursorFile`

```ts
type CursorFile = {
  filePath: string;
  fileName: string;
  previewPath: string | null;
  previewDataUrl: string | null;
  type: "CUR" | "ANI";
  exists: boolean;
};
```

未分配文件必须位于当前皮肤的 `storagePath` 内部。`assign_unassigned_cursor` 会再次规范化和校验路径，前端传入相同文件名并不等于拥有文件访问权。

同一内部光标文件可以暂时被多个角色引用。替换或交换时，只有在旧文件没有被其他角色引用且未重复列入未分配列表时，才把它加入 `unassignedFiles`。

## 预览字段

### `previewPath`

- 指向 `.previews/` 中由 Rust 生成的 PNG。
- 文件名包含清理后的来源文件名、稳定哈希和 `.centered-v1.png` 后缀。
- 哈希和后缀用于避免重名冲突，并允许预览算法版本变化。
- 缓存缺失时可以重新生成，不是原始皮肤数据。
- 成功替换或分配后会清理不再引用的缓存文件。

### `previewDataUrl`

- 是返回 React 前由后端补充的瞬态显示数据。
- 持久化前会被清除，因此 `library.json` 中通常为 `null` 或不存在于旧数据。
- 不得把 base64 data URL 当作光标文件来源，也不要将其写回数据库。

ANI 预览只表示首个可解析帧，不改变原 `.ani` 文件或 Windows 中的动画效果。

## `settings.json`

当前格式为：

```json
{
  "closeToTray": true
}
```

| 字段 | 类型 | 默认值 | 含义 |
| --- | --- | --- | --- |
| `closeToTray` | `boolean` | `true` | 关闭主窗口时隐藏到托盘，而不是结束进程 |

开机自启状态由 Tauri autostart 插件和 Windows 系统项维护，不保存在 `settings.json` 中。

## 写入与备份

`library.json` 和 `settings.json` 都由 Rust 后端写入：

1. 在同目录创建临时文件。
2. 写入格式化 JSON 并同步文件内容。
3. 将现有有效主文件复制为对应 `.bak.json`。
4. 用临时文件替换主文件。
5. 替换失败时尝试恢复备份。

主文件无法解析时会读取备份。主文件和备份都损坏时，后端保留带时间标记的损坏副本，创建空库或默认设置，并把原因写入日志。

不要在应用运行时手工编辑 JSON。前端操作和手工修改可能互相覆盖，而且错误的绝对路径可能让清理操作指向错误位置。故障排查需要试验旧数据时，应先从托盘彻底退出应用，再复制整个数据目录并在隔离环境中操作。

## 兼容性规则

当前格式没有显式 `schemaVersion`。因此数据结构变更必须遵守：

- 新增可选字段时使用 Rust `serde(default)` 或等价迁移，确保旧文件可读。
- 重命名、删除字段或改变类型时提供显式迁移，不能只修改结构体。
- 路径迁移必须同时处理 `storagePath`、角色文件、未分配文件和预览路径。
- 旧版 Tauri 数据目录迁移后必须重写内部路径，并保持用户原始来源不变。
- 任何迁移都需要覆盖有效主文件、仅备份有效、损坏文件和中途写入失败。
- JSON 字段使用 camelCase；Rust 内部字段保持 snake_case，通过 Serde 映射。

如果未来引入不兼容格式，应先增加 `schemaVersion`、版本化迁移和降级策略，再发布产生新格式的二进制。

## 数据不变量

每次后端操作完成后应满足：

- 一个皮肤恰好有 15 个角色，`windowsKey` 唯一。
- 已分配和未分配路径都属于当前皮肤内部目录。
- 外部 `sourcePath` 不作为 Windows 注册表光标值。
- 文件类型只允许可验证的 CUR 或 ANI，扩展名比较不区分大小写。
- 新复制文件不覆盖已有文件。
- 未分配列表不重复包含相同内部路径。
- 被多个角色共享的文件不会因单一角色替换而删除或重复分配。
- `isApplied=true` 仅表示所有已设置角色当前都与注册表一致。
- 数据库写入失败时，本次替换或分配的映射和新增文件得到回滚。

对这些不变量的修改属于高风险变更，应同时更新 Rust 测试、前端状态测试、本文和架构文档。
