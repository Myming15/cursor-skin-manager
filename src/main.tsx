import React, { useEffect, useMemo, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { getVersion } from "@tauri-apps/api/app";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowRightLeft, Check, FilePenLine, Settings as SettingsIcon, X } from "lucide-react";
import "./styles.css";

export type CursorRole = {
  role: string;
  windowsKey: string;
  filePath: string | null;
  fileName: string | null;
  previewPath: string | null;
  previewDataUrl: string | null;
  type: "cur" | "ani" | null;
  exists: boolean;
};

export type CursorFile = {
  filePath: string;
  fileName: string;
  previewPath: string | null;
  previewDataUrl: string | null;
  type: "cur" | "ani";
  exists: boolean;
};

export type SkinPackage = {
  id: string;
  name: string;
  sourcePath: string;
  storagePath: string;
  importedAt: string;
  hasInf: boolean;
  isComplete: boolean;
  cursorCount: number;
  isApplied: boolean;
  importNote?: string | null;
  roles: CursorRole[];
  unassignedFiles: CursorFile[];
};

type ActionFeedback = { kind: "applied"; skinId: string } | { kind: "reset" } | null;

type AppSettings = {
  closeToTray: boolean;
};

type CursorOperation =
  { kind: "replace"; windowsKey: string } | { kind: "assign"; filePath: string } | null;

const CURSOR_EDIT_REAPPLY_MESSAGE = "皮肤已修改，请重新点击‘应用此皮肤’使更改生效。";
const TOAST_AUTO_DISMISS_MS = 3200;

const ROLE_PREVIEWS: Record<string, string> = {
  Arrow: "↖",
  Help: "?",
  AppStarting: "◌",
  Wait: "◷",
  Crosshair: "+",
  IBeam: "I",
  NWPen: "✎",
  No: "⊘",
  SizeNS: "↕",
  SizeWE: "↔",
  SizeNWSE: "↖↘",
  SizeNESW: "↙↗",
  SizeAll: "✥",
  UpArrow: "↑",
  Hand: "☝",
};

const demoSkins: SkinPackage[] = [
  {
    id: "demo-fluent-dark",
    name: "Windows 11 Fluent Dark",
    sourcePath: "Downloads/Fluent-Dark-Cursors.zip",
    storagePath: "%AppData%/CursorSkinManager/skins/demo-fluent-dark",
    importedAt: "2026-07-08T12:00:00+08:00",
    hasInf: true,
    isComplete: true,
    cursorCount: 15,
    isApplied: true,
    importNote: null,
    roles: [
      ["Normal Select", "Arrow", "arrow.cur", "cur"],
      ["Help Select", "Help", "help.cur", "cur"],
      ["Working In Background", "AppStarting", "working.ani", "ani"],
      ["Busy", "Wait", "busy.ani", "ani"],
      ["Precision Select", "Crosshair", "cross.cur", "cur"],
      ["Text Select", "IBeam", "beam.cur", "cur"],
      ["Handwriting", "NWPen", "pen.cur", "cur"],
      ["Unavailable", "No", "no.cur", "cur"],
      ["Vertical Resize", "SizeNS", "size_ns.cur", "cur"],
      ["Horizontal Resize", "SizeWE", "size_we.cur", "cur"],
      ["Diagonal Resize 1", "SizeNWSE", "size_nwse.cur", "cur"],
      ["Diagonal Resize 2", "SizeNESW", "size_nesw.cur", "cur"],
      ["Move", "SizeAll", "move.cur", "cur"],
      ["Alternate Select", "UpArrow", "up.cur", "cur"],
      ["Link Select", "Hand", "hand.cur", "cur"],
    ].map(([role, windowsKey, fileName, type]) => ({
      role,
      windowsKey,
      filePath: `demo/${fileName}`,
      fileName,
      type: type as "cur" | "ani",
      previewPath: null,
      previewDataUrl: null,
      exists: true,
    })),
    unassignedFiles: [],
  },
];

const isTauri = "__TAURI_INTERNALS__" in window;

export function App() {
  const [skins, setSkins] = useState<SkinPackage[]>(isTauri ? [] : demoSkins);
  const [selectedId, setSelectedId] = useState<string | null>(isTauri ? null : demoSkins[0].id);
  const [message, setMessage] = useState(
    isTauri
      ? "正在读取本地皮肤库..."
      : "浏览器预览模式：导入、删除、应用需要在 Tauri 桌面窗口中运行。"
  );
  const [busy, setBusy] = useState(false);
  const [actionFeedback, setActionFeedback] = useState<ActionFeedback>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsBusy, setSettingsBusy] = useState(false);
  const [launchAtStartup, setLaunchAtStartup] = useState(false);
  const [closeToTray, setCloseToTray] = useState(true);
  const [appVersion, setAppVersion] = useState("0.1.11");
  const [settingsMessage, setSettingsMessage] = useState("");
  const [cursorOperation, setCursorOperation] = useState<CursorOperation>(null);
  const [assignmentFile, setAssignmentFile] = useState<CursorFile | null>(null);
  const [assignmentRoleKey, setAssignmentRoleKey] = useState<string | null>(null);
  const feedbackTimer = useRef<number | null>(null);

  const selectedSkin = useMemo(
    () => skins.find((skin) => skin.id === selectedId) ?? skins[0] ?? null,
    [selectedId, skins]
  );
  const selectedHasBrokenFiles = selectedSkin ? hasBrokenFiles(selectedSkin) : false;

  useEffect(() => {
    if (!isTauri) return;
    refreshLibrary();
    loadSettings();
    getVersion()
      .then(setAppVersion)
      .catch(() => setAppVersion("0.1.11"));
  }, []);

  useEffect(() => {
    return () => {
      if (feedbackTimer.current !== null) {
        window.clearTimeout(feedbackTimer.current);
      }
    };
  }, []);

  useEffect(() => {
    setAssignmentFile(null);
    setAssignmentRoleKey(null);
  }, [selectedId]);

  function showActionFeedback(feedback: Exclude<ActionFeedback, null>) {
    if (feedbackTimer.current !== null) {
      window.clearTimeout(feedbackTimer.current);
    }
    setActionFeedback(feedback);
    feedbackTimer.current = window.setTimeout(() => {
      setActionFeedback(null);
      feedbackTimer.current = null;
    }, 1600);
  }

  async function refreshLibrary(nextSelectedId?: string) {
    try {
      const library = await invoke<SkinPackage[]>("load_library");
      setSkins(library);
      setSelectedId(nextSelectedId ?? library[0]?.id ?? null);
      setMessage(
        library.length > 0 ? "" : "还没有皮肤包，请先导入本地文件夹、.zip、.cur 或 .ani。"
      );
    } catch (error) {
      setMessage(`读取失败：${String(error)}`);
    }
  }

  async function importSkin(kind: "directory" | "file") {
    if (!isTauri) {
      setMessage("当前是浏览器预览模式，请用 npm run tauri dev 打开桌面窗口后再导入。");
      return;
    }

    const selected = await open({
      title: kind === "directory" ? "选择皮肤包文件夹" : "选择皮肤包文件",
      multiple: false,
      directory: kind === "directory",
      filters:
        kind === "directory"
          ? undefined
          : [
              { name: "Cursor skins", extensions: ["zip", "cur", "ani", "inf"] },
              { name: "All files", extensions: ["*"] },
            ],
    });

    if (typeof selected !== "string") return;

    setBusy(true);
    setMessage("正在导入并识别皮肤包...");
    try {
      const skin = await invoke<SkinPackage>("import_skin", { sourcePath: selected });
      setSkins((current) => [skin, ...current]);
      setSelectedId(skin.id);
      setMessage(skin.cursorCount > 0 ? "" : "未找到 .cur / .ani 文件，请选择有效的皮肤包。");
    } catch (error) {
      setMessage(`导入失败：${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function replaceCursorRole(skin: SkinPackage, cursor: CursorRole) {
    if (!isTauri) {
      setMessage("浏览器预览模式不能替换光标文件。");
      return;
    }
    if (busy) return;

    setBusy(true);
    setCursorOperation({ kind: "replace", windowsKey: cursor.windowsKey });
    setMessage("");
    try {
      const selected = await open({
        title: `为 ${cursor.role} 选择光标文件`,
        multiple: false,
        directory: false,
        filters: [{ name: "Windows 光标文件", extensions: ["cur", "ani"] }],
      });
      if (typeof selected !== "string") return;

      setMessage(`正在验证并替换 ${cursor.role}...`);
      const wasApplied = skin.isApplied;
      const library = await invoke<SkinPackage[]>("replace_cursor_role", {
        skinId: skin.id,
        windowsKey: cursor.windowsKey,
        sourcePath: selected,
      });
      setSkins(library);
      setSelectedId(skin.id);
      setMessage(wasApplied ? CURSOR_EDIT_REAPPLY_MESSAGE : `${cursor.role} 已替换。`);
    } catch (error) {
      setMessage(`替换失败：${String(error)}`);
    } finally {
      setCursorOperation(null);
      setBusy(false);
    }
  }

  function openAssignmentDialog(file: CursorFile) {
    if (busy || !file.exists) return;
    setAssignmentFile(file);
    setAssignmentRoleKey(null);
  }

  function closeAssignmentDialog() {
    if (busy) return;
    setAssignmentFile(null);
    setAssignmentRoleKey(null);
  }

  async function assignUnassignedCursor() {
    if (!isTauri || !selectedSkin || !assignmentFile || !assignmentRoleKey || busy) return;

    const wasApplied = selectedSkin.isApplied;
    const targetRole = selectedSkin.roles.find((role) => role.windowsKey === assignmentRoleKey);
    setBusy(true);
    setCursorOperation({ kind: "assign", filePath: assignmentFile.filePath });
    setMessage(`正在将 ${assignmentFile.fileName} 分配到 ${targetRole?.role ?? "目标角色"}...`);
    try {
      const library = await invoke<SkinPackage[]>("assign_unassigned_cursor", {
        skinId: selectedSkin.id,
        sourceFilePath: assignmentFile.filePath,
        windowsKey: assignmentRoleKey,
      });
      setSkins(library);
      setSelectedId(selectedSkin.id);
      setAssignmentFile(null);
      setAssignmentRoleKey(null);
      setMessage(
        wasApplied
          ? CURSOR_EDIT_REAPPLY_MESSAGE
          : `${assignmentFile.fileName} 已分配到 ${targetRole?.role ?? "目标角色"}。`
      );
    } catch (error) {
      setMessage(`分配失败：${String(error)}`);
    } finally {
      setCursorOperation(null);
      setBusy(false);
    }
  }

  async function deleteSkin(skin: SkinPackage) {
    if (!isTauri) {
      setMessage("浏览器预览模式不会删除文件。");
      return;
    }

    if (skin.isApplied) {
      const confirmed = window.confirm(
        "该皮肤当前正在使用。删除前会先恢复 Windows 默认光标，然后再删除该皮肤包。是否继续？"
      );
      if (!confirmed) return;
    }

    setBusy(true);
    try {
      await invoke("delete_skin", { skinId: skin.id });
      const next = skins.filter((item) => item.id !== skin.id);
      setSkins(next);
      setSelectedId(next[0]?.id ?? null);
      setMessage("");
    } catch (error) {
      setMessage(`删除失败：${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function applySkin(skin: SkinPackage) {
    if (!isTauri) {
      setMessage("浏览器预览模式不能修改 Windows 当前用户光标。");
      return;
    }

    if (!skin.isComplete) {
      const confirmed = window.confirm(
        `该皮肤包只匹配到 ${matchedCount(skin)} / 15 个光标角色。应用后，未设置的项目将保持当前系统配置不变。是否继续？`
      );
      if (!confirmed) return;
    }
    if (hasBrokenFiles(skin)) {
      setMessage("应用内部保存的部分光标文件已经缺失，请重新导入该皮肤包。");
      return;
    }

    setBusy(true);
    setMessage("正在写入当前 Windows 用户光标配置...");
    try {
      const library = await invoke<SkinPackage[]>("apply_skin", { skinId: skin.id });
      setSkins(library);
      setSelectedId(skin.id);
      setMessage("");
      showActionFeedback({ kind: "applied", skinId: skin.id });
    } catch (error) {
      setMessage(`应用失败：${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function refreshCursors() {
    if (!isTauri) {
      setMessage("浏览器预览模式不能刷新 Windows 光标。");
      return;
    }

    setBusy(true);
    try {
      await invoke("refresh_system_cursors");
      setMessage("");
    } catch (error) {
      setMessage(`刷新失败：${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function resetDefaultCursors() {
    if (!isTauri) {
      setMessage("浏览器预览模式不能恢复 Windows 默认光标。");
      return;
    }

    const confirmed = window.confirm(
      "确定要恢复 Windows 默认鼠标光标吗？这会覆盖当前用户的光标设置。"
    );
    if (!confirmed) return;

    setBusy(true);
    setMessage("正在恢复 Windows 默认光标...");
    try {
      const library = await invoke<SkinPackage[]>("reset_system_cursors");
      setSkins(library);
      setSelectedId((current) => current ?? library[0]?.id ?? null);
      setMessage("");
      showActionFeedback({ kind: "reset" });
      if (settingsOpen) setSettingsMessage("Windows 默认光标已恢复。");
    } catch (error) {
      const errorMessage = `恢复默认失败：${String(error)}`;
      if (settingsOpen) {
        setSettingsMessage(errorMessage);
      } else {
        setMessage(errorMessage);
      }
    } finally {
      setBusy(false);
    }
  }

  async function loadSettings() {
    try {
      const [storedSettings, autostartEnabled] = await Promise.all([
        invoke<AppSettings>("load_app_settings"),
        isAutostartEnabled(),
      ]);
      setCloseToTray(storedSettings.closeToTray);
      setLaunchAtStartup(autostartEnabled);
    } catch (error) {
      setSettingsMessage(`读取设置失败：${String(error)}`);
    }
  }

  async function changeAutostart(enabled: boolean) {
    setSettingsBusy(true);
    setSettingsMessage("");
    try {
      if (enabled) {
        await enableAutostart();
      } else {
        await disableAutostart();
      }
      setLaunchAtStartup(await isAutostartEnabled());
    } catch (error) {
      setSettingsMessage(`修改开机自启失败：${String(error)}`);
    } finally {
      setSettingsBusy(false);
    }
  }

  async function changeCloseToTray(enabled: boolean) {
    setSettingsBusy(true);
    setSettingsMessage("");
    try {
      const settings = await invoke<AppSettings>("set_close_to_tray", { enabled });
      setCloseToTray(settings.closeToTray);
    } catch (error) {
      setSettingsMessage(`保存关闭行为失败：${String(error)}`);
    } finally {
      setSettingsBusy(false);
    }
  }

  async function clearAllSkins() {
    const confirmed = window.confirm(
      "确定要清空全部本地皮肤吗？此操作会删除应用内保存的所有皮肤包。"
    );
    if (!confirmed) return;
    const confirmedAgain = window.confirm(
      "此操作无法撤销。若当前有正在使用的皮肤，会先恢复 Windows 默认光标。确认继续？"
    );
    if (!confirmedAgain) return;

    setSettingsBusy(true);
    setBusy(true);
    setSettingsMessage("");
    try {
      const library = await invoke<SkinPackage[]>("clear_all_skins");
      setSkins(library);
      setSelectedId(null);
      setSettingsMessage("全部本地皮肤已清空。");
    } catch (error) {
      setSettingsMessage(`清空失败：${String(error)}`);
    } finally {
      setSettingsBusy(false);
      setBusy(false);
    }
  }

  async function openSkinDir(skin: SkinPackage) {
    if (!isTauri) {
      setMessage("浏览器预览模式不能打开皮肤目录。");
      return;
    }
    try {
      await invoke("open_skin_dir", { skinId: skin.id });
      setMessage("");
    } catch (error) {
      setMessage(`打开皮肤目录失败：${String(error)}`);
    }
  }

  async function openLogFile() {
    if (!isTauri) {
      setMessage("浏览器预览模式不能打开日志。");
      return;
    }
    try {
      await invoke("open_log_file");
      setMessage("");
      if (settingsOpen) setSettingsMessage("");
    } catch (error) {
      const errorMessage = `打开日志失败：${String(error)}`;
      if (settingsOpen) {
        setSettingsMessage(errorMessage);
      } else {
        setMessage(errorMessage);
      }
    }
  }

  return (
    <main className="app-shell">
      <section className="window" aria-label="Cursor Skin Manager MVP">
        <div className="layout">
          <aside className="sidebar">
            <section className="import-box">
              <div className="import-actions">
                <button
                  className="primary-button"
                  type="button"
                  onClick={() => importSkin("directory")}
                  disabled={busy}
                >
                  导入文件夹
                </button>
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => importSkin("file")}
                  disabled={busy}
                >
                  导入文件
                </button>
              </div>
              <p>支持文件夹、.zip、.cur、.ani、包含 install.inf 的皮肤包。</p>
            </section>

            <div className="section-title">
              <span>本地皮肤包</span>
              <span>{skins.length} 个</span>
            </div>

            <div className="skin-list">
              {skins.length === 0 ? (
                <div className="empty-state">暂无皮肤包</div>
              ) : (
                skins.map((skin) => (
                  <button
                    className={`skin-item ${skin.id === selectedSkin?.id ? "active" : ""}`}
                    key={skin.id}
                    type="button"
                    onClick={() => setSelectedId(skin.id)}
                  >
                    <div className="skin-icon">
                      <CursorPreview
                        cursor={skinListPreview(skin)}
                        fallback={
                          ROLE_PREVIEWS[
                            skin.roles.find((role) => role.exists)?.windowsKey ?? "Arrow"
                          ] ?? "↖"
                        }
                      />
                    </div>
                    <div className="skin-copy">
                      <h2>{skin.name}</h2>
                      <div className="pill-row">
                        {skin.isApplied && <span className="pill ok">正在使用</span>}
                        <span className="pill">{skin.cursorCount} 个光标</span>
                        {skin.hasInf ? (
                          <span className="pill">INF</span>
                        ) : (
                          <span className="pill warn">文件名识别</span>
                        )}
                        {hasBrokenFiles(skin) && <span className="pill danger">文件缺失</span>}
                        {!skin.isComplete && <span className="pill warn">不完整</span>}
                      </div>
                    </div>
                  </button>
                ))
              )}
            </div>

            <div className="sidebar-settings-footer">
              <button
                className="sidebar-settings-button"
                type="button"
                title="设置"
                aria-label="打开设置"
                onClick={() => {
                  setSettingsMessage("");
                  setSettingsOpen(true);
                }}
              >
                <SettingsIcon size={19} strokeWidth={2} aria-hidden="true" />
              </button>
            </div>
          </aside>

          <section className="content">
            {selectedSkin ? (
              <>
                <div className="toolbar">
                  <div>
                    <h1>{selectedSkin.name}</h1>
                    <p className="subtitle">
                      来源：{selectedSkin.sourcePath} ·{" "}
                      {selectedSkin.hasInf ? "已解析安装配置" : "根据文件名识别"}
                    </p>
                  </div>
                  <div className="actions">
                    <button
                      className="secondary-button"
                      type="button"
                      onClick={() => openSkinDir(selectedSkin)}
                      disabled={busy}
                    >
                      打开皮肤目录
                    </button>
                    <button
                      className="secondary-button"
                      type="button"
                      onClick={refreshCursors}
                      disabled={busy}
                    >
                      重新加载光标
                    </button>
                    <button
                      className="danger-button"
                      type="button"
                      onClick={() => deleteSkin(selectedSkin)}
                      disabled={busy}
                    >
                      删除
                    </button>
                    <button
                      className="primary-button"
                      type="button"
                      onClick={() => applySkin(selectedSkin)}
                      disabled={
                        busy ||
                        selectedSkin.cursorCount === 0 ||
                        matchedCount(selectedSkin) === 0 ||
                        selectedHasBrokenFiles
                      }
                    >
                      {actionFeedback?.kind === "applied" &&
                      actionFeedback.skinId === selectedSkin.id
                        ? "已应用"
                        : "应用此皮肤"}
                    </button>
                  </div>
                </div>

                <div className="stats-grid">
                  <StatCard
                    label="识别方式"
                    value={selectedSkin.hasInf ? "install.inf" : "文件名"}
                  />
                  <StatCard label="光标文件" value={`${selectedSkin.cursorCount} 个`} />
                  <StatCard label="完整度" value={`${matchedCount(selectedSkin)} / 15`} />
                  <StatCard label="当前状态" value={selectedSkin.isApplied ? "已应用" : "未应用"} />
                </div>

                <div className="preview-header">
                  <h2>整套光标预览</h2>
                  <span>静态 .cur 优先显示真实预览，动态 .ani 尝试显示首帧</span>
                </div>

                <div className="cursor-grid">
                  {selectedSkin.roles.map((cursor) => (
                    <CursorRoleCard
                      key={cursor.windowsKey}
                      cursor={cursor}
                      disabled={busy}
                      processing={
                        cursorOperation?.kind === "replace" &&
                        cursorOperation.windowsKey === cursor.windowsKey
                      }
                      onReplace={() => replaceCursorRole(selectedSkin, cursor)}
                    />
                  ))}
                </div>

                {selectedSkin.unassignedFiles.length > 0 && (
                  <section className="unassigned-panel">
                    <div className="preview-header">
                      <h2>未分配文件</h2>
                      <span>这些光标文件已导入，但没有匹配到 Windows 光标角色。</span>
                    </div>
                    <div className="unassigned-list">
                      {selectedSkin.unassignedFiles.map((file) => (
                        <UnassignedCursorCard
                          key={file.filePath}
                          file={file}
                          disabled={busy || !file.exists}
                          processing={
                            cursorOperation?.kind === "assign" &&
                            cursorOperation.filePath === file.filePath
                          }
                          onAssign={() => openAssignmentDialog(file)}
                        />
                      ))}
                    </div>
                  </section>
                )}
              </>
            ) : (
              <div className="detail-empty">
                <h1>导入一个本地光标皮肤包</h1>
                <p>
                  选择文件夹、.zip、.cur、.ani 或包含 install.inf
                  的安装包，应用会复制到内部目录并自动识别光标角色。
                </p>
                <div className="empty-actions">
                  <button
                    className="primary-button"
                    type="button"
                    onClick={() => importSkin("directory")}
                    disabled={busy}
                  >
                    导入文件夹
                  </button>
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => importSkin("file")}
                    disabled={busy}
                  >
                    导入文件
                  </button>
                </div>
              </div>
            )}

            <AppToast message={message} onDismiss={() => setMessage("")} />
          </section>
        </div>
      </section>

      {assignmentFile && selectedSkin && (
        <RoleAssignmentDialog
          skin={selectedSkin}
          file={assignmentFile}
          selectedRoleKey={assignmentRoleKey}
          busy={busy}
          onSelectRole={setAssignmentRoleKey}
          onCancel={closeAssignmentDialog}
          onConfirm={assignUnassignedCursor}
        />
      )}

      {settingsOpen && (
        <div
          className="settings-backdrop"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setSettingsOpen(false);
          }}
        >
          <section
            className="settings-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="settings-title"
          >
            <header className="settings-header">
              <h2 id="settings-title">设置</h2>
              <button type="button" onClick={() => setSettingsOpen(false)} aria-label="关闭设置">
                关闭
              </button>
            </header>

            <div className="settings-section">
              <h3>常规</h3>
              <div className="setting-row">
                <div>
                  <strong>开机自启</strong>
                  <span>登录 Windows 后自动启动应用。</span>
                </div>
                <label className="switch-control">
                  <input
                    type="checkbox"
                    checked={launchAtStartup}
                    onChange={(event) => changeAutostart(event.target.checked)}
                    disabled={settingsBusy}
                  />
                  <span aria-hidden="true" />
                </label>
              </div>
              <div className="setting-row">
                <div>
                  <strong>关闭窗口时最小化到托盘</strong>
                  <span>关闭窗口后应用继续在系统托盘运行，可从托盘重新打开或退出。</span>
                </div>
                <label className="switch-control">
                  <input
                    type="checkbox"
                    checked={closeToTray}
                    onChange={(event) => changeCloseToTray(event.target.checked)}
                    disabled={settingsBusy}
                  />
                  <span aria-hidden="true" />
                </label>
              </div>
            </div>

            <div className="settings-section">
              <h3>数据与维护</h3>
              <div className="setting-action-row">
                <div>
                  <strong>恢复默认光标</strong>
                  <span>将当前用户的鼠标光标恢复为 Windows 默认方案。</span>
                </div>
                <button type="button" onClick={resetDefaultCursors} disabled={settingsBusy || busy}>
                  {actionFeedback?.kind === "reset" ? "已恢复" : "恢复默认"}
                </button>
              </div>
              <div className="setting-action-row">
                <div>
                  <strong>应用日志</strong>
                  <span>打开运行日志，查看导入、应用和系统刷新记录。</span>
                </div>
                <button type="button" onClick={openLogFile}>
                  打开日志
                </button>
              </div>
              <div className="danger-setting">
                <div>
                  <strong>清空全部皮肤</strong>
                  <p>
                    会删除应用内保存的全部本地皮肤；若当前皮肤正在使用，将先恢复 Windows
                    默认光标。此操作无法撤销。
                  </p>
                </div>
                <button type="button" onClick={clearAllSkins} disabled={settingsBusy || busy}>
                  清空全部皮肤
                </button>
              </div>
            </div>

            <div className="settings-section settings-about-section">
              <h3>关于</h3>
              <div className="setting-info-row">
                <div>
                  <strong>Cursor Skin Manager</strong>
                  <span>当前安装版本</span>
                </div>
                <span className="version-value">v{appVersion}</span>
              </div>
            </div>

            {settingsMessage && <p className="settings-message">{settingsMessage}</p>}
          </section>
        </div>
      )}
    </main>
  );
}

export function CursorRoleCard({
  cursor,
  disabled,
  processing,
  onReplace,
}: {
  cursor: CursorRole;
  disabled: boolean;
  processing: boolean;
  onReplace: () => void;
}) {
  const bluePreview =
    cursor.exists && ["Arrow", "Help", "AppStarting", "Wait", "Hand"].includes(cursor.windowsKey);

  return (
    <div
      className={`cursor-card ${cursor.exists ? "" : "missing"} ${disabled ? "disabled" : ""} ${processing ? "processing" : ""}`}
    >
      <button
        className="cursor-card-action"
        type="button"
        onClick={onReplace}
        disabled={disabled}
        aria-label={`替换 ${cursor.role} 光标文件`}
      >
        <FilePenLine size={13} strokeWidth={2} />
        {processing ? "处理中..." : "替换文件"}
      </button>
      <span className={`cursor-preview ${bluePreview ? "blue" : ""}`}>
        <CursorPreview cursor={cursor} fallback={ROLE_PREVIEWS[cursor.windowsKey] ?? "↖"} />
      </span>
      <span className="cursor-card-copy">
        <strong>{cursor.role}</strong>
        <span className="cursor-file-meta">
          <span title={cursor.fileName ?? "未设置"}>
            {cursor.exists ? cursor.fileName : "未设置"}
          </span>
          {cursor.exists && cursor.type && (
            <span className="cursor-type">{cursor.type.toUpperCase()}</span>
          )}
        </span>
      </span>
    </div>
  );
}

export function UnassignedCursorCard({
  file,
  disabled,
  processing,
  onAssign,
}: {
  file: CursorFile;
  disabled: boolean;
  processing: boolean;
  onAssign: () => void;
}) {
  return (
    <div
      className={`unassigned-item ${file.exists ? "" : "missing"} ${disabled ? "disabled" : ""} ${processing ? "processing" : ""}`}
    >
      <span className="mini-preview">
        <CursorPreview cursor={file} fallback={file.type.toUpperCase()} />
      </span>
      <span className="unassigned-copy">
        <strong title={file.fileName}>{file.fileName}</strong>
        <span>{file.exists ? file.type.toUpperCase() : "文件缺失"}</span>
      </span>
      <button
        className="unassigned-action"
        type="button"
        onClick={onAssign}
        disabled={disabled}
        aria-label={`将 ${file.fileName} 分配到光标角色`}
      >
        <ArrowRightLeft size={13} strokeWidth={2} />
        {processing ? "处理中..." : "分配到角色"}
      </button>
    </div>
  );
}

export function AppToast({ message, onDismiss }: { message: string; onDismiss: () => void }) {
  const dismissRef = useRef(onDismiss);

  useEffect(() => {
    dismissRef.current = onDismiss;
  }, [onDismiss]);

  useEffect(() => {
    if (!shouldAutoDismissToast(message)) return;
    const timer = window.setTimeout(() => dismissRef.current(), TOAST_AUTO_DISMISS_MS);
    return () => window.clearTimeout(timer);
  }, [message]);

  if (!message) return null;
  return (
    <div className="toast" role="status" aria-live="polite">
      {message}
    </div>
  );
}

export function shouldAutoDismissToast(message: string) {
  return (
    message === CURSOR_EDIT_REAPPLY_MESSAGE ||
    message.endsWith(" 已替换。") ||
    (message.includes(" 已分配到 ") && message.endsWith("。"))
  );
}

export function RoleAssignmentDialog({
  skin,
  file,
  selectedRoleKey,
  busy,
  onSelectRole,
  onCancel,
  onConfirm,
}: {
  skin: SkinPackage;
  file: CursorFile;
  selectedRoleKey: string | null;
  busy: boolean;
  onSelectRole: (windowsKey: string) => void;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const latestState = useRef({ busy, onCancel });
  latestState.current = { busy, onCancel };

  useEffect(() => {
    const previousFocus =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const focusTimer = window.setTimeout(() => {
      dialogRef.current?.querySelector<HTMLElement>("[data-role-option]")?.focus();
    }, 0);

    function handleKeyDown(event: KeyboardEvent) {
      const dialog = dialogRef.current;
      if (!dialog) return;
      if (event.key === "Escape") {
        if (!latestState.current.busy) {
          event.preventDefault();
          latestState.current.onCancel();
        }
        return;
      }
      if (event.key !== "Tab") return;

      const focusable = Array.from(
        dialog.querySelectorAll<HTMLElement>(
          'button:not([disabled]), [href], input:not([disabled]), [tabindex]:not([tabindex="-1"])'
        )
      );
      if (focusable.length === 0) {
        event.preventDefault();
        dialog.focus();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => {
      window.clearTimeout(focusTimer);
      document.removeEventListener("keydown", handleKeyDown);
      previousFocus?.focus();
    };
  }, []);

  return (
    <div
      className="assignment-backdrop"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy) onCancel();
      }}
    >
      <div
        className="assignment-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="assignment-title"
        tabIndex={-1}
        ref={dialogRef}
      >
        <header className="assignment-header">
          <div>
            <h2 id="assignment-title">分配到角色</h2>
            <p>选择一个 Windows 光标角色并确认替换。</p>
          </div>
          <button
            className="icon-button"
            type="button"
            onClick={onCancel}
            disabled={busy}
            aria-label="关闭角色选择"
          >
            <X size={18} strokeWidth={2} aria-hidden="true" />
          </button>
        </header>

        <div className="assignment-source">
          <span className="mini-preview">
            <CursorPreview cursor={file} fallback={file.type.toUpperCase()} />
          </span>
          <span>
            <strong title={file.fileName}>{file.fileName}</strong>
            <span>{file.type.toUpperCase()} · 将从未分配列表移出</span>
          </span>
        </div>

        <div className="assignment-role-grid" role="radiogroup" aria-label="Windows 光标角色">
          {skin.roles.map((role) => {
            const selected = role.windowsKey === selectedRoleKey;
            return (
              <button
                className={`assignment-role ${selected ? "selected" : ""}`}
                type="button"
                role="radio"
                aria-checked={selected}
                data-role-option
                disabled={busy}
                key={role.windowsKey}
                onClick={() => onSelectRole(role.windowsKey)}
              >
                <span className="assignment-role-preview">
                  <CursorPreview cursor={role} fallback={ROLE_PREVIEWS[role.windowsKey] ?? "↖"} />
                </span>
                <span className="assignment-role-copy">
                  <strong>{role.role}</strong>
                  <span title={role.fileName ?? "未设置"}>
                    {role.exists ? role.fileName : "未设置"}
                  </span>
                  <small className={role.exists ? "is-set" : ""}>
                    {role.exists ? "已设置" : "未设置"}
                  </small>
                </span>
                <span className="assignment-role-check" aria-hidden="true">
                  {selected && <Check size={16} strokeWidth={2.4} />}
                </span>
              </button>
            );
          })}
        </div>

        <footer className="assignment-footer">
          <button className="secondary-button" type="button" onClick={onCancel} disabled={busy}>
            取消
          </button>
          <button
            className="primary-button"
            type="button"
            onClick={onConfirm}
            disabled={busy || !selectedRoleKey}
          >
            {busy ? "正在分配..." : "确认替换"}
          </button>
        </footer>
      </div>
    </div>
  );
}

function matchedCount(skin: SkinPackage) {
  return skin.roles.filter((role) => role.exists).length;
}

function hasBrokenFiles(skin: SkinPackage) {
  return (
    skin.roles.some((role) => role.filePath && !role.exists) ||
    skin.unassignedFiles.some((file) => !file.exists)
  );
}

function skinListPreview(skin: SkinPackage): {
  previewPath: string | null;
  previewDataUrl: string | null;
  exists: boolean;
} {
  return (
    skin.roles.find((role) => role.exists && (role.previewDataUrl || role.previewPath)) ??
    skin.unassignedFiles.find(
      (file) => file.exists && (file.previewDataUrl || file.previewPath)
    ) ?? {
      previewPath: null,
      previewDataUrl: null,
      exists: false,
    }
  );
}

export function CursorPreview({
  cursor,
  fallback,
}: {
  cursor: { previewPath: string | null; previewDataUrl: string | null; exists: boolean };
  fallback: string;
}) {
  const [failed, setFailed] = useState(false);
  const src =
    cursor.previewDataUrl ?? (cursor.previewPath ? convertFileSrc(cursor.previewPath) : null);

  useEffect(() => {
    setFailed(false);
  }, [src]);

  if (cursor.exists && src && !failed) {
    return <img src={src} alt="" onError={() => setFailed(true)} />;
  }

  return <span>{fallback}</span>;
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <article className="stat-card">
      <span>{label}</span>
      <strong>{value}</strong>
    </article>
  );
}

const rootElement = document.getElementById("root");
if (rootElement) {
  ReactDOM.createRoot(rootElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}
