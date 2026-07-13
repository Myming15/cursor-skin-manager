# Windows 鼠标指针皮肤管理器 MVP 文档

## 1. 产品定位

做一个简单、单一、好用的 Windows 桌面应用，用于管理用户从各类网站下载到本地的鼠标指针皮肤包。

MVP 只解决一个核心问题：

> 用户把本地光标皮肤包导入应用后，可以预览、应用、删除。

本阶段不做在线皮肤商店、不做账号、不做社区、不做皮肤编辑器、不做云同步。

## 2. 目标用户

- 喜欢 Windows 桌面美化的用户
- 经常从致美化、DeviantArt、RealWorld Cursor Library、GitHub、论坛等地方下载鼠标指针包的用户
- 不想每次都手动右键安装 `.inf`、打开控制面板、逐项选择光标的用户

## 3. MVP 功能范围

### 必须实现

1. 导入本地皮肤包
2. 自动识别 `.cur` / `.ani` / `.inf`
3. 预览整套光标
4. 一键应用到 Windows 当前用户
5. 删除皮肤包

### 明确不做

- 在线皮肤市场
- 用户登录
- 收藏、标签、评分
- 皮肤编辑器
- 光标格式转换
- 多设备同步
- 自动从网页抓取皮肤
- 复杂动画特效
- 系统级全用户安装

## 4. 核心用户流程

### 4.1 导入皮肤包

用户打开应用，点击「导入皮肤包」。

支持导入：

- 单个 `.cur` 文件
- 单个 `.ani` 文件
- 包含多个 `.cur` / `.ani` 的文件夹
- 包含 `.inf` 的文件夹
- `.zip` 压缩包

MVP 可暂不支持 `.rar` / `.7z`，除非后续集成解压库。

导入后，应用自动分析文件内容，生成一个皮肤包条目。

### 4.2 自动识别

应用扫描导入内容：

- 如果存在 `.inf`，优先解析 `.inf`
- 如果不存在 `.inf`，扫描所有 `.cur` / `.ani`
- 如果只有一个光标文件，也允许导入，但标记为“不完整皮肤包”

识别成功后，在主界面显示皮肤包名称、预览图和包含的光标数量。

### 4.3 预览整套光标

用户点击某个皮肤包后，进入详情区域。

详情区域显示 Windows 常见光标角色：

- Normal Select
- Help Select
- Working In Background
- Busy
- Precision Select
- Text Select
- Handwriting
- Unavailable
- Vertical Resize
- Horizontal Resize
- Diagonal Resize 1
- Diagonal Resize 2
- Move
- Alternate Select
- Link Select

每个角色显示：

- 角色名称
- 当前匹配到的 `.cur` / `.ani` 文件名
- 光标预览

如果某个角色没有匹配文件，显示「未设置」。

### 4.4 一键应用到 Windows 当前用户

用户点击「应用此皮肤」。

应用将该皮肤包对应的光标方案写入 Windows 当前用户配置。

MVP 只影响当前登录用户，不要求管理员权限。

应用成功后：

- 更新当前用户的鼠标指针注册表配置
- 通知系统刷新鼠标指针
- 在应用内显示“已应用”

如果皮肤包不完整：

- 可允许应用已有角色
- 未提供的角色保持当前系统设置不变
- 应用前给出提示：“该皮肤包缺少部分光标，未设置的项目将保持不变。”

### 4.5 删除皮肤包

用户点击某个皮肤包的「删除」。

删除行为：

- 从应用资料库中移除该皮肤包
- 删除应用复制保存的皮肤文件
- 不主动删除 Windows 系统目录中的文件
- 如果该皮肤正在使用，删除前提示用户

提示文案：

> 该皮肤当前正在使用。删除后，已应用到系统的光标可能仍会保留，但应用内将不再管理它。是否继续？

MVP 不强制恢复默认光标。

## 5. 数据模型

### 5.1 SkinPackage

```json
{
  "id": "uuid",
  "name": "皮肤包名称",
  "sourcePath": "原始导入路径",
  "storagePath": "应用内部保存路径",
  "importedAt": "2026-07-08T12:00:00+08:00",
  "hasInf": true,
  "isComplete": false,
  "cursorCount": 12,
  "roles": []
}
```

### 5.2 CursorRole

```json
{
  "role": "Normal Select",
  "windowsKey": "Arrow",
  "filePath": "storage/skin-id/aero_arrow.cur",
  "fileName": "aero_arrow.cur",
  "type": "cur",
  "exists": true
}
```

## 6. Windows 光标角色映射

应用内部需要维护 Windows 光标角色与注册表字段的映射。

| 显示名称 | 注册表字段 |
|---|---|
| Normal Select | Arrow |
| Help Select | Help |
| Working In Background | AppStarting |
| Busy | Wait |
| Precision Select | Crosshair |
| Text Select | IBeam |
| Handwriting | NWPen |
| Unavailable | No |
| Vertical Resize | SizeNS |
| Horizontal Resize | SizeWE |
| Diagonal Resize 1 | SizeNWSE |
| Diagonal Resize 2 | SizeNESW |
| Move | SizeAll |
| Alternate Select | UpArrow |
| Link Select | Hand |

## 7. `.inf` 解析规则

MVP 只需要支持常见 Windows 光标主题 `.inf`。

解析目标：

- 读取皮肤名称
- 读取每个角色对应的 `.cur` / `.ani` 文件
- 识别需要复制的文件列表

优先关注 `.inf` 中的这些区域：

- `[Version]`
- `[DefaultInstall]`
- `[DestinationDirs]`
- `[Scheme.Cur]`
- `[Scheme.Reg]`
- `[Strings]`

如果 `.inf` 解析失败：

- 降级为普通文件夹扫描
- 根据文件名猜测角色
- 在 UI 中显示“未能完整解析安装配置”

## 8. 无 `.inf` 时的自动匹配规则

当导入内容没有 `.inf` 时，根据文件名进行简单匹配。

示例：

| 文件名关键词 | 匹配角色 |
|---|---|
| arrow, normal, pointer | Arrow |
| help | Help |
| appstarting, working | AppStarting |
| wait, busy, loading | Wait |
| cross, crosshair | Crosshair |
| text, ibeam, beam | IBeam |
| pen, handwriting | NWPen |
| no, unavailable, forbidden | No |
| ns, vert, vertical | SizeNS |
| we, horz, horizontal | SizeWE |
| nwse | SizeNWSE |
| nesw | SizeNESW |
| move, all | SizeAll |
| up, alternate | UpArrow |
| hand, link | Hand |

匹配不到的文件进入“未分配”区域。

## 9. 文件保存策略

导入时，应用应复制一份文件到自己的数据目录，避免原始文件被用户移动或删除后皮肤失效。

推荐目录：

```text
%AppData%/CursorSkinManager/skins/{skinId}/
```

每个皮肤包独立存放。

应用数据库可使用：

```text
%AppData%/CursorSkinManager/library.json
```

MVP 使用 JSON 文件即可，不需要数据库。

## 10. 应用到 Windows 的技术方案

Windows 当前用户光标配置通常位于：

```text
HKEY_CURRENT_USER\Control Panel\Cursors
```

应用皮肤时，写入对应字段：

```text
Arrow
Help
AppStarting
Wait
Crosshair
IBeam
NWPen
No
SizeNS
SizeWE
SizeNWSE
SizeNESW
SizeAll
UpArrow
Hand
```

写入值为 `.cur` / `.ani` 文件的绝对路径。

写入完成后，需要通知系统刷新光标配置。

可通过 Windows API：

```text
SystemParametersInfo(SPI_SETCURSORS, 0, null, SPIF_UPDATEINIFILE | SPIF_SENDCHANGE)
```

MVP 应只写入 `HKEY_CURRENT_USER`，避免管理员权限和系统级风险。

## 11. 界面结构

### 11.1 主界面

主界面分为两栏：

- 左侧：皮肤包列表
- 右侧：选中皮肤包详情与预览

### 11.2 顶部操作

- 导入皮肤包

### 11.3 皮肤包列表项

显示：

- 皮肤名称
- 光标数量
- 是否完整
- 是否当前正在使用

操作：

- 选择
- 删除

### 11.4 详情区域

显示：

- 皮肤名称
- 来源信息
- 光标角色预览网格
- 应用此皮肤按钮
- 删除按钮

## 12. 状态与提示

### 导入成功

> 已导入皮肤包。

### 未找到光标文件

> 未找到 `.cur` / `.ani` 文件，请选择有效的鼠标指针皮肤包。

### `.inf` 解析失败

> 未能完整解析安装配置，已尝试根据文件名识别光标。

### 应用成功

> 已应用到当前 Windows 用户。

### 应用失败

> 应用失败，请确认光标文件仍然存在。

### 删除成功

> 已删除皮肤包。

## 13. 异常场景

### 13.1 导入重复皮肤

MVP 可允许重复导入。

重复导入时生成新的皮肤包 ID。

### 13.2 光标文件缺失

如果应用内部保存的文件被手动删除：

- 预览显示文件缺失
- 禁用应用按钮
- 提示用户重新导入

### 13.3 动态光标预览

`.ani` 动态预览不是 MVP 的强要求。

最低要求：

- 能显示静态占位预览
- 标记为 ANI 动态光标

更好方案：

- 播放 `.ani` 动画预览

### 13.4 应用后系统未立即刷新

如果调用刷新 API 后仍未生效：

- 提示用户重新打开鼠标设置或注销后再查看
- 记录错误日志

## 14. 验收标准

### 导入本地皮肤包

- 用户可以选择文件夹或 `.zip`
- 应用能复制皮肤文件到内部目录
- 应用重启后皮肤包仍存在

### 自动识别 `.cur` / `.ani` / `.inf`

- 能识别 `.cur`
- 能识别 `.ani`
- 能识别常见 `.inf` 光标安装文件
- `.inf` 失败时能降级扫描文件

### 预览整套光标

- 能显示所有 Windows 常见光标角色
- 能显示每个角色是否已匹配文件
- 能显示 `.cur` 静态预览
- `.ani` 至少显示文件存在和类型

### 一键应用到 Windows 当前用户

- 点击一次即可应用皮肤
- 只修改当前用户配置
- 不需要管理员权限
- 应用后 Windows 光标发生变化

### 删除皮肤包

- 可从列表删除皮肤包
- 删除后应用重启不再显示
- 对应内部文件被删除
- 删除当前使用中的皮肤包时有确认提示

## 15. MVP 成功标准

用户可以完成以下完整闭环：

1. 从任意网站下载一个 Windows 鼠标指针包
2. 在应用中导入
3. 看到整套光标预览
4. 点击一次应用到当前 Windows 用户
5. 不喜欢时从应用中删除

只要这个闭环顺畅，MVP 就成立。
