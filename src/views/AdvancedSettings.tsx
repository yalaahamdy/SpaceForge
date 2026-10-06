/** Advanced Tools (spec §59), Windows maintenance ops, Settings (spec §52, §49),
 *  global search (spec §43), and report export (spec §51). */

import React, { useEffect, useState } from "react";
import { ipc } from "../api/ipc";
import type {
  AppSettings, ComponentStoreInfo, Exclusion, FileEntry, MaintenanceInfo,
  ScanMode, VolumeInfo,
} from "../api/types";
import { formatBytes, formatCount, formatDateTime } from "../lib/format";
import { useApp } from "../state/AppState";
import {
  Column, ContextMenu, EmptyState, usePathActions, VirtualTable, Modal,
} from "../components/common/UI";
import { IconSearch } from "../components/common/icons";

// ---------------------------------------------------------------------------
// Advanced tools

export function AdvancedView() {
  const { lastScan, toast, elevated, refreshElevation } = useApp();
  const [maint, setMaint] = useState<MaintenanceInfo | null>(null);
  const [compStore, setCompStore] = useState<ComponentStoreInfo | null>(null);
  const [analyzing, setAnalyzing] = useState(false);
  const [restoreInfo, setRestoreInfo] = useState<string | null>(null);
  const [exportOpen, setExportOpen] = useState(false);

  useEffect(() => {
    ipc.advancedMaintenance().then(setMaint).catch(() => setMaint(null));
  }, [lastScan]);

  async function analyze() {
    setAnalyzing(true);
    try {
      const info = await ipc.componentStoreAnalyze();
      setCompStore(info);
      await refreshElevation();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setAnalyzing(false);
    }
  }

  async function restoreStorage() {
    try {
      setRestoreInfo(await ipc.restoreStorageInfo());
    } catch (e) {
      toast(String(e), "error");
    }
  }

  return (
    <div>
      <h1 className="page-title">Advanced Tools</h1>
      <p className="page-sub">
        Expert-level diagnostics. These tools never reduce the quality of the normal experience —
        they exist for the “show me everything” moments.
      </p>

      {!elevated && (
        <div className="warning-box" style={{ marginBottom: 14 }}>
          Running without administrator privileges: DISM analysis, Delivery Optimization and
          shadow-copy tools are limited. Restart SpaceForge as administrator for full access.
        </div>
      )}

      <div className="grid-2">
        <div className="card">
          <h3>Windows component store (WinSxS)</h3>
          <p className="dim" style={{ fontSize: 12.5 }}>
            Serviced exclusively through the supported DISM mechanism — files are never touched directly.
          </p>
          <button className="btn small" onClick={() => void analyze()} disabled={analyzing}>
            {analyzing ? "Analyzing (may take a minute)…" : "Analyze component store"}
          </button>
          {compStore && (
            <div style={{ marginTop: 10, fontSize: 13 }}>
              <div>Actual size: <b className="num">{formatBytes(compStore.actualSizeBytes)}</b></div>
              <div>Cleanup recommended: <b>{compStore.cleanupRecommended ? "yes" : "no"}</b></div>
              <div className="dim3" style={{ fontSize: 11.5, marginTop: 6 }}>
                Run the “Component Store cleanup (DISM)” rule in Windows Cleanup to act on this.
                Cleanup can affect update rollback capability — DISM explains the trade-off.
              </div>
            </div>
          )}
        </div>

        <div className="card">
          <h3>Hibernation &amp; recovery</h3>
          {maint ? (
            <div style={{ fontSize: 13 }}>
              <div>
                Hibernation: <b>{maint.hibernationEnabled ? "enabled" : "disabled"}</b>
                {maint.hibernationEnabled && <> — hiberfil.sys ≈ <b className="num">{formatBytes(maint.hiberfilSize)}</b></>}
              </div>
              <div className="dim" style={{ fontSize: 12, margin: "6px 0 10px" }}>
                Disabling hibernation (powercfg /h off) also disables Fast Startup. The rule lives
                in Windows Cleanup — the file is never deleted manually.
              </div>
              <div>Recycle Bin: <b className="num">{formatCount(maint.recycleBinItems)} items · {formatBytes(maint.recycleBinBytes)}</b></div>
              <button className="btn small" style={{ marginTop: 10 }} onClick={() => void restoreStorage()}>
                Query shadow-copy storage (vssadmin)
              </button>
            </div>
          ) : (
            <p className="dim" style={{ fontSize: 12.5 }}>Windows-only information.</p>
          )}
          {restoreInfo && (
            <pre className="mono dim" style={{
              marginTop: 10, fontSize: 11, whiteSpace: "pre-wrap",
              maxHeight: 180, overflowY: "auto", userSelect: "text",
            }}>{restoreInfo}</pre>
          )}
        </div>
      </div>

      <div className="card" style={{ marginTop: 14 }}>
        <div className="row">
          <div>
            <h3 style={{ margin: 0 }}>Export report</h3>
            <p className="dim" style={{ fontSize: 12.5, margin: 0 }}>
              Self-contained HTML, CSV or JSON from the current scan. Nothing leaves this PC.
            </p>
          </div>
          <div className="spacer" />
          <button className="btn" onClick={() => setExportOpen(true)} disabled={!lastScan}>
            Export…
          </button>
        </div>
      </div>

      <ExportDialog open={exportOpen} onClose={() => setExportOpen(false)} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Export dialog

function ExportDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { toast } = useApp();
  const [format, setFormat] = useState("html");
  const [includePaths, setIncludePaths] = useState(false);
  if (!open) return null;

  async function doExport() {
    try {
      const savedPath = await ipc.exportReport(format, "auto", includePaths);
      toast(`Report saved to ${savedPath}`, "ok");
      onClose();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  return (
    <Modal title="Export analysis report" onClose={onClose} footer={
      <>
        <button className="btn" onClick={onClose}>Cancel</button>
        <button className="btn primary" onClick={() => void doExport()}>Export</button>
      </>
    }>
      <div className="stack" style={{ gap: 10, fontSize: 13.5 }}>
        <label className="row">
          <input type="radio" checked={format === "html"} onChange={() => setFormat("html")} />
          HTML — attractive, self-contained report
        </label>
        <label className="row">
          <input type="radio" checked={format === "csv"} onChange={() => setFormat("csv")} />
          CSV — tabular data for spreadsheets
        </label>
        <label className="row">
          <input type="radio" checked={format === "json"} onChange={() => setFormat("json")} />
          JSON — full structured data
        </label>
        <label className="row">
          <input type="checkbox" checked={includePaths} onChange={(e) => setIncludePaths(e.target.checked)} />
          Include file paths (leave off for a privacy-safe summary)
        </label>
        <p className="dim3" style={{ fontSize: 12 }}>
          The report is written to your Downloads folder. Diagnostic export is always an explicit
          user action — SpaceForge never transmits anything.
        </p>
      </div>
    </Modal>
  );
}

// ---------------------------------------------------------------------------
// Global search (spec §43)

export function SearchPanel({ externalQuery }: { externalQuery?: string }) {
  const { lastScan, scanning } = useApp();
  const [ownQuery, setOwnQuery] = useState("");
  const query = externalQuery ?? ownQuery;
  const setQuery = setOwnQuery;
  const [rows, setRows] = useState<FileEntry[]>([]);
  const [menu, setMenu] = useState<{ x: number; y: number; path: string } | null>(null);
  const pathActions = usePathActions();

  useEffect(() => {
    if (!lastScan) return;
    const t = window.setTimeout(() => {
      if (query.trim().length >= 2) {
        ipc.searchFiles(query.trim(), 0, 100).then(setRows).catch(() => setRows([]));
      } else {
        setRows([]);
      }
    }, 180);
    return () => window.clearTimeout(t);
  }, [query, lastScan]);

  if (scanning) return null;
  if (!lastScan || query.trim().length < 2) return null;

  const cols: Column<FileEntry>[] = [
    { key: "name", label: "File", width: "minmax(220px,1fr)",
      render: (f) => <span title={f.path}>{f.name}</span> },
    { key: "path", label: "Path", width: "minmax(220px,1.1fr)",
      render: (f) => <span className="dim mono">{f.path}</span> },
    { key: "size", label: "Size", width: "95px", align: "right", sortValue: (f) => f.size,
      render: (f) => <span className="num">{formatBytes(f.size)}</span> },
    { key: "mod", label: "Modified", width: "115px", sortValue: (f) => f.modified ?? "",
      render: (f) => <span className="dim">{formatDateTime(f.modified)}</span> },
  ];

  return (
    <div className="card" style={{ marginTop: 14 }}>
      <h3 className="row"><IconSearch size={14} /> Search results for “{query}”</h3>
      <div style={{ maxHeight: 320 }}>
        <VirtualTable rows={rows} columns={cols} onRowContext={(row, x, y) => setMenu({ x, y, path: row.path })} />
      </div>
      {menu && (
        <ContextMenu x={menu.x} y={menu.y} onClose={() => setMenu(null)} items={[
          { label: "Open containing folder", action: () => void pathActions(menu.path, "folder") },
          { label: "Copy path", action: () => void pathActions(menu.path, "copy") },
        ]} />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Settings (spec §52)

export function SettingsView() {
  const { settings, saveSettings, toast, volumes, selectedVolume } = useApp();
  const [local, setLocal] = useState<AppSettings>(settings);
  const [exclusions, setExclusions] = useState<Exclusion[]>([]);
  const [newPattern, setNewPattern] = useState("");

  useEffect(() => { setLocal(settings); }, [settings]);
  useEffect(() => {
    ipc.listExclusions().then(setExclusions).catch(() => setExclusions([]));
  }, []);

  function update<K extends keyof AppSettings>(key: K, value: AppSettings[K]) {
    setLocal((s) => ({ ...s, [key]: value }));
  }

  async function persist() {
    try {
      await saveSettings(local);
      toast("Settings saved.", "ok");
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function addExclusion() {
    if (!newPattern.trim()) return;
    try {
      const ex = await ipc.addExclusion(newPattern.trim(), "user-added");
      setExclusions((e) => [ex, ...e]);
      setNewPattern("");
    } catch (e) {
      toast(String(e), "error");
    }
  }

  const Row: React.FC<{ label: string; hint?: string; children: React.ReactNode }> = ({ label, hint, children }) => (
    <div className="row" style={{ padding: "9px 0", borderBottom: "1px solid var(--stroke-1)" }}>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontWeight: 600, fontSize: 13.5 }}>{label}</div>
        {hint && <div className="dim3" style={{ fontSize: 12 }}>{hint}</div>}
      </div>
      <div className="spacer" />
      {children}
    </div>
  );

  return (
    <div>
      <div className="row-wrap">
        <h1 className="page-title" style={{ margin: 0 }}>Settings</h1>
        <div className="spacer" />
        <button className="btn primary" onClick={() => void persist()}>Save changes</button>
      </div>
      <p className="page-sub">Everything is stored locally. Telemetry does not exist; the toggle below is fixed off by design.</p>

      <div className="grid-2">
        <div className="card">
          <h3>Scanning</h3>
          <Row label="Default scan mode" hint="Balanced is recommended">
            <select value={local.scanMode} onChange={(e) => update("scanMode", e.target.value as ScanMode)}>
              <option value="battery_saver">Battery Saver</option>
              <option value="balanced">Balanced</option>
              <option value="fast">Fast</option>
              <option value="max_performance">Maximum Performance</option>
            </select>
          </Row>
          <Row label="Index threshold" hint="Files below this size are aggregated (10 MB default)">
            <select value={local.indexThresholdBytes} onChange={(e) => update("indexThresholdBytes", Number(e.target.value))}>
              <option value={1024 * 1024}>1 MB</option>
              <option value={10 * 1024 * 1024}>10 MB</option>
              <option value={50 * 1024 * 1024}>50 MB</option>
            </select>
          </Row>
          <Row label="Traverse reparse points" hint="Junctions/symlinks — increases loop risk; off by default">
            <input type="checkbox" checked={local.traverseReparsePoints}
              onChange={(e) => update("traverseReparsePoints", e.target.checked)} />
          </Row>
          <Row label="Scan network drives" hint="Slower and less reliable by nature">
            <input type="checkbox" checked={local.scanNetworkDrives}
              onChange={(e) => update("scanNetworkDrives", e.target.checked)} />
          </Row>
          <Row label="Experimental NTFS MFT scan" hint="WizTree-class speed; requires admin; falls back safely">
            <input type="checkbox" checked={local.enableExperimentalMft}
              onChange={(e) => update("enableExperimentalMft", e.target.checked)} />
          </Row>
        </div>

        <div className="card">
          <h3>Cleanup</h3>
          <Row label="Require confirmation before cleaning">
            <input type="checkbox" checked={local.confirmBeforeClean}
              onChange={(e) => update("confirmBeforeClean", e.target.checked)} />
          </Row>
          <Row label="Prefer Recycle Bin for user files" hint="Restorable deletion where appropriate">
            <input type="checkbox" checked={local.preferRecycleBin}
              onChange={(e) => update("preferRecycleBin", e.target.checked)} />
          </Row>
          <Row label="Enable opt-in cleaners" hint="Developer registry caches, privacy history/cookies">
            <input type="checkbox" checked={local.enableOptInCleaners}
              onChange={(e) => update("enableOptInCleaners", e.target.checked)} />
          </Row>
          <Row label="Duplicates: preferred folder" hint="Folder to prefer when picking the keeper copy">
            <input type="text" value={local.preferredDuplicatesFolder ?? ""}
              placeholder="e.g. D:\\Keep"
              onChange={(e) => update("preferredDuplicatesFolder", e.target.value || null)} />
          </Row>
        </div>

        <div className="card">
          <h3>Privacy</h3>
          <Row label="Telemetry" hint="Hard-disabled: SpaceForge is local-only by architecture">
            <input type="checkbox" checked={false} disabled />
          </Row>
          <Row label="Debug logging" hint="Verbose local logs for scanner diagnostics">
            <input type="checkbox" checked={local.debugLogging}
              onChange={(e) => update("debugLogging", e.target.checked)} />
          </Row>
          <Row label="Background auto-scan" hint="Scan + notify; never deletes automatically">
            <input type="checkbox" checked={local.backgroundAutoscan}
              onChange={(e) => update("backgroundAutoscan", e.target.checked)} />
          </Row>
        </div>

        <div className="card">
          <h3>Exclusions</h3>
          <div className="row" style={{ marginBottom: 8 }}>
            <input type="text" placeholder="D:\\Games\\** or *.iso" value={newPattern}
              onChange={(e) => setNewPattern(e.target.value)} style={{ flex: 1 }} />
            <button className="btn small" onClick={() => void addExclusion()}>Add</button>
          </div>
          {exclusions.length === 0 ? (
            <p className="dim3" style={{ fontSize: 12.5 }}>
              No exclusions. Excluded paths are skipped by scans and duplicate detection.
            </p>
          ) : (
            exclusions.map((x) => (
              <div key={x.id} className="row" style={{ padding: "5px 0", fontSize: 12.5 }}>
                <span className="mono">{x.pattern}</span>
                <div className="spacer" />
                <span className="dim3">{formatDateTime(x.createdAt)}</span>
                <button className="btn small ghost" onClick={() => {
                  void ipc.removeExclusion(x.id).then(() =>
                    setExclusions((e) => e.filter((y) => y.id !== x.id)));
                }}>Remove</button>
              </div>
            ))
          )}
        </div>
      </div>

      <div className="card" style={{ marginTop: 14 }}>
        <h3>Current volume target</h3>
        <p className="dim" style={{ fontSize: 13, margin: 0 }}>
          Dashboard scans target <b>{selectedVolume || "—"}</b> from{" "}
          {volumes.length} detected volume(s). Change the target by selecting a drive card on the Dashboard.
        </p>
      </div>
    </div>
  );
}
