/** File Types view (spec §16) + Duplicates view (spec §19). */

import React, { useEffect, useMemo, useRef, useState } from "react";
import { ipc, listen } from "../api/ipc";
import type { DuplicateGroup, KeepStrategy, TopConsumers } from "../api/types";
import { formatBytes, formatCount, ageFrom } from "../lib/format";
import { colorFor, labelFor, CATEGORY_COLORS } from "../lib/categoryColor";
import { useApp } from "../state/AppState";
import {
  CategoryLegend, Donut, EmptyState, usePathActions, VirtualTable, Column,
  ContextMenu, Modal,
} from "../components/common/UI";
import { IconDuplicates, IconPlay } from "../components/common/icons";
import { FileExplorerIcon } from "../components/common/FileExplorerIcon";

// ---------------------------------------------------------------------------
// File types

export function FileTypesView() {
  const { lastScan, scanning } = useApp();
  const [data, setData] = useState<TopConsumers | null>(null);

  useEffect(() => {
    if (!lastScan) return;
    ipc.fileTypes().then(setData).catch(() => setData(null));
  }, [lastScan]);

  if (scanning) return <EmptyState title="Scanning…" />;
  if (!lastScan) return <EmptyState title="No scan available" hint="File-type rollups come from the latest scan." />;
  if (!data) return <EmptyState title="Nothing to show" />;

  const donutData = data.topCategories.map(([cat, size]) => ({
    label: labelFor(cat), value: size, color: colorFor(cat),
  }));
  const totalSize = data.topCategories.reduce((a, b) => a + b[1], 0);

  return (
    <div>
      <h1 className="page-title">File Types</h1>
      <p className="page-sub">What kinds of content consume your disk — by category and extension.</p>

      <div className="grid-2">
        <div className="card">
          <h3>By category — {formatBytes(totalSize)}</h3>
          <div className="row" style={{ alignItems: "flex-start" }}>
            <Donut data={donutData} />
            <div className="spacer" />
            <div style={{ minWidth: 260 }}>
              <CategoryLegend data={data.topCategories.map(([c, s, n]) => [c, s, n])} />
            </div>
          </div>
        </div>
        <div className="card">
          <h3>Top extensions</h3>
          <div style={{ maxHeight: 320, overflowY: "auto" }}>
            {data.topExtensions.slice(0, 20).map(([ext, size, count]) => (
              <div key={ext || "(none)"} className="row" style={{ padding: "4px 0", fontSize: 13 }}>
                <span className="mono">{ext || "(no extension)"}</span>
                <div className="spacer" />
                <span className="num" style={{ width: 90, textAlign: "right" }}>{formatBytes(size)}</span>
                <span className="num dim3" style={{ width: 80, textAlign: "right" }}>{formatCount(count)} files</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="card" style={{ marginTop: 14 }}>
        <h3>Largest folders (top 50)</h3>
        <div className="grid-2">
          {data.topFolders.slice(0, 20).map(([path, size, files]) => (
            <div key={path} className="row" style={{ fontSize: 12.5 }}>
              <span className="mono dim" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={path}>
                {path}
              </span>
              <div className="spacer" />
              <span className="num">{formatBytes(size)}</span>
              <span className="dim3 num" style={{ width: 70, textAlign: "right" }}>{formatCount(files)}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Duplicates

interface DupProgress { stage: string; done: number; total: number }

export function DuplicatesView() {
  const { lastScan, scanning, toast, settings } = useApp();
  const [groups, setGroups] = useState<DuplicateGroup[] | null>(null);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<DupProgress | null>(null);
  const [strategy, setStrategy] = useState<KeepStrategy>("keep_newest");
  const [minSize, setMinSize] = useState(100 * 1024 * 1024);
  const [openGroup, setOpenGroup] = useState<DuplicateGroup | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; path: string } | null>(null);
  const pathActions = usePathActions();
  const cancelled = useRef(false);

  useEffect(() => {
    const un1 = listen<DupProgress>("duplicates-progress", setProgress);
    return () => { void un1.then((u) => u()); };
  }, []);

  async function run() {
    setRunning(true);
    setGroups(null);
    cancelled.current = false;
    try {
      const res = await ipc.findDuplicates({
        useScan: true,
        extraFolders: [],
        minSize,
        keepStrategy: strategy,
        preferredFolder: settings.preferredDuplicatesFolder ?? undefined,
      });
      setGroups(res);
      const waste = res.reduce((a, b) => a + b.wasted, 0);
      toast(res.length === 0
        ? "No duplicate content found — everything is unique."
        : `${res.length} duplicate groups found · ${formatBytes(waste)} redundant.`,
        res.length === 0 ? "ok" : "info");
    } catch (e) {
      if (!cancelled.current) toast(String(e), "error");
    } finally {
      setRunning(false);
      setProgress(null);
    }
  }

  if (scanning) return <EmptyState title="Scanning…" />;
  if (!lastScan) return <EmptyState title="No scan available" hint="Duplicate detection runs over the latest scan index (plus optional extra folders)." />;

  const wasted = groups?.reduce((a, b) => a + b.wasted, 0) ?? 0;

  return (
    <div>
      <div className="row-wrap">
        <h1 className="page-title" style={{ margin: 0 }}>Duplicates</h1>
        <div className="spacer" />
        <label className="dim" htmlFor="dup-strategy">Keep</label>
        <select id="dup-strategy" value={strategy} onChange={(e) => setStrategy(e.target.value as KeepStrategy)}>
          <option value="keep_newest">newest copy</option>
          <option value="keep_oldest">oldest copy</option>
          <option value="keep_shortest_path">shortest path</option>
          <option value="keep_first">recommended</option>
        </select>
        <label className="dim" htmlFor="dup-min">Min size</label>
        <select id="dup-min" value={minSize} onChange={(e) => setMinSize(Number(e.target.value))}>
          <option value={1024 * 1024}>1 MB</option>
          <option value={10 * 1024 * 1024}>10 MB</option>
          <option value={100 * 1024 * 1024}>100 MB</option>
          <option value={1024 * 1024 * 1024}>1 GB</option>
        </select>
        <button className="btn primary" onClick={() => void run()} disabled={running}>
          <IconPlay size={13} /> {running ? "Searching…" : "Find duplicates"}
        </button>
        {running && (
          <button className="btn danger" onClick={() => { cancelled.current = true; void ipc.cancelDuplicates(); }}>
            Cancel
          </button>
        )}
      </div>
      <p className="page-sub">
        Multi-stage detection: same size → partial hash → full hash → byte-by-byte verification.
        Hard links are one identity and never counted as redundant copies. Nothing is deleted
        without explicit confirmation.
      </p>

      {running && progress && (
        <div className="card">
          <div className="row">
            <b style={{ fontSize: 13 }}>{progress.stage} pass</b>
            <div className="spacer" />
            <span className="dim num">{formatCount(progress.done)} / {formatCount(progress.total)}</span>
          </div>
          <div className="progress-line" style={{ marginTop: 8 }}>
            <div style={{ width: progress.total > 0 ? `${(progress.done / progress.total) * 100}%` : "5%" }} />
          </div>
        </div>
      )}

      {groups === null && !running && (
        <EmptyState
          icon={<IconDuplicates size={38} />}
          title="No duplicate search yet"
          hint="Search the current scan for files with identical content, grouped by wasted space."
          action={<button className="btn primary" onClick={() => void run()}>Find duplicates</button>}
        />
      )}

      {groups && groups.length === 0 && (
        <EmptyState icon={<IconDuplicates size={38} />}
          title="No duplicate groups found" hint="Your storage is looking good — no redundant copies at this threshold." />
      )}

      {groups && groups.length > 0 && (
        <div className="card">
          <div className="row" style={{ marginBottom: 10 }}>
            <h3 style={{ margin: 0 }}>{formatCount(groups.length)} groups · {formatBytes(wasted)} redundant</h3>
            <div className="spacer" />
            <span className="dim" style={{ fontSize: 12 }}>Sorted by wasted space</span>
          </div>
          <div className="stack" style={{ gap: 8 }}>
            {groups.map((g) => (
              <div key={g.id} className="card" style={{ padding: 12 }}>
                <div className="row" style={{ gap: 8 }}>
                  <FileExplorerIcon extension={g.keep.split(".").pop() || ""} size={18} />
                  <span className="mono" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={g.keep}>
                    {g.keep.split(/[\\/]/).pop()}
                  </span>
                  <span className="badge low">{formatBytes(g.sizeEach)} each</span>
                  <span className="badge review">{g.copies} copies</span>
                  <span className="badge dangerous">{formatBytes(g.wasted)} wasted</span>
                  <div className="spacer" />
                  <button className="btn small ghost" onClick={() => setOpenGroup(g)}>Review copies →</button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {openGroup && (
        <Modal
          title={`Duplicate group — ${formatBytes(openGroup.wasted)} redundant`}
          onClose={() => setOpenGroup(null)}
          footer={<button className="btn" onClick={() => setOpenGroup(null)}>Close</button>}
        >
          <p className="dim" style={{ fontSize: 13 }}>
            The keeper is preselected by your strategy. To remove a redundant copy, use its
            context menu — deletion always asks for confirmation.
          </p>
          {openGroup.paths.map((p, i) => (
            <div key={p.path} className="row" style={{ padding: "6px 0", borderBottom: "1px solid var(--stroke-1)", gap: 8 }}>
              <span className={`badge ${i === 0 ? "safe" : "review"}`}>{i === 0 ? "KEEP" : "COPY"}</span>
              <FileExplorerIcon extension={p.path.split(".").pop() || ""} size={16} />
              <span className="mono" style={{ fontSize: 11.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={p.path}>
                {p.path}
              </span>
              <div className="spacer" />
              <span className="dim">{ageFrom(p.modified)}</span>
              {i !== 0 && (
                <button className="btn small danger"
                  onClick={() => { void pathActions(p.path, "recycle"); }}>
                  Recycle
                </button>
              )}
            </div>
          ))}
        </Modal>
      )}

      {menu && (
        <ContextMenu x={menu.x} y={menu.y} onClose={() => setMenu(null)} items={[
          { label: "Open containing folder", action: () => void pathActions(menu.path, "folder") },
          { label: "Copy path", action: () => void pathActions(menu.path, "copy") },
        ]} />
      )}
    </div>
  );
}
