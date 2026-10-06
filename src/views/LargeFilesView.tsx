/** Large Files finder (spec §17) — thresholds, filters, honest labeling. */

import React, { useEffect, useState } from "react";
import { ipc } from "../api/ipc";
import type { FileEntry } from "../api/types";
import { formatBytes, ageFrom, formatCount } from "../lib/format";
import { labelFor } from "../lib/categoryColor";
import { useApp } from "../state/AppState";
import { Column, ContextMenu, EmptyState, usePathActions, VirtualTable } from "../components/common/UI";
import { FileExplorerIcon } from "../components/common/FileExplorerIcon";

const THRESHOLDS = [100 * 1024 * 1024, 500 * 1024 * 1024, 1024 ** 3, 5 * 1024 ** 3, 10 * 1024 ** 3];

export function LargeFilesView() {
  const { lastScan, scanning, settings } = useApp();
  const [minSize, setMinSize] = useState(settings.largeFileThreshold);
  const [olderThanDays, setOlderThanDays] = useState<number | "">("");
  const [category, setCategory] = useState("all");
  const [rows, setRows] = useState<FileEntry[]>([]);
  const [menu, setMenu] = useState<{ x: number; y: number; row: FileEntry } | null>(null);
  const pathActions = usePathActions();

  useEffect(() => {
    let alive = true;
    ipc.largeFiles({
      minSize,
      olderThanDays: olderThanDays === "" ? undefined : olderThanDays,
      category: category === "all" ? undefined : category,
      limit: 500,
    }).then((r) => alive && setRows(r)).catch(() => alive && setRows([]));
    return () => { alive = false; };
  }, [lastScan, minSize, olderThanDays, category]);

  if (scanning) return <EmptyState title="Scanning…" />;
  if (!lastScan) return <EmptyState title="No scan available" hint="Large-file discovery works on scan data. Run a scan first." />;

  const total = rows.reduce((a, b) => a + b.size, 0);

  const cols: Column<FileEntry>[] = [
    {
      key: "name", label: "File", width: "minmax(260px,1fr)", sortValue: (f) => f.name.toLowerCase(),
      render: (f) => (
        <span className="row" style={{ gap: 8 }}>
          <FileExplorerIcon category={f.category} extension={f.extension} size={18} />
          <span title={f.path} style={{ overflow: "hidden", textOverflow: "ellipsis" }}>{f.name}</span>
        </span>
      ),
    },
    {
      key: "path", label: "Location", width: "minmax(200px,0.9fr)", sortValue: (f) => f.path,
      render: (f) => <span className="dim mono" title={f.path}>{f.path}</span>,
    },
    {
      key: "size", label: "Size", width: "100px", align: "right", sortValue: (f) => f.size,
      render: (f) => <span className="num">{formatBytes(f.size)}</span>,
    },
    {
      key: "cat", label: "Category", width: "110px", sortValue: (f) => f.category,
      render: (f) => <span className="dim">{labelFor(f.category)}</span>,
    },
    {
      key: "modified", label: "Modified", width: "110px", sortValue: (f) => f.modified ?? "",
      render: (f) => <span className="dim">{ageFrom(f.modified)}</span>,
    },
  ];

  return (
    <div style={{ height: "calc(100vh - 120px)", display: "flex", flexDirection: "column" }}>
      <h1 className="page-title">Large Files</h1>
      <p className="page-sub">
        Large does not mean disposable — these are candidates to review, not junk to purge.
        {rows.length > 0 && <> Total shown: <b className="num">{formatBytes(total)}</b> across {formatCount(rows.length)} files.</>}
      </p>

      <div className="row-wrap" style={{ marginBottom: 10 }}>
        <div className="seg" role="group" aria-label="Size threshold">
          {THRESHOLDS.map((t) => (
            <button key={t} className={minSize === t ? "active" : ""} onClick={() => setMinSize(t)}>
              ≥ {formatBytes(t)}
            </button>
          ))}
        </div>
        <label className="dim" htmlFor="lf-age">Older than</label>
        <select id="lf-age" value={olderThanDays} onChange={(e) => setOlderThanDays(e.target.value === "" ? "" : Number(e.target.value))}>
          <option value="">any age</option>
          <option value={30}>30 days</option>
          <option value={90}>90 days</option>
          <option value={180}>180 days</option>
          <option value={365}>1 year</option>
        </select>
        <label className="dim" htmlFor="lf-cat">Category</label>
        <select id="lf-cat" value={category} onChange={(e) => setCategory(e.target.value)}>
          <option value="all">all</option>
          {["video", "audio", "image", "archive", "disk_image", "executable", "game", "system", "developer", "unknown"].map((c) => (
            <option key={c} value={c}>{labelFor(c)}</option>
          ))}
        </select>
      </div>

      <div style={{ flex: 1, minHeight: 0 }}>
        <VirtualTable
          rows={rows}
          columns={cols}
          defaultSort="size"
          onRowContext={(row, x, y) => setMenu({ x, y, row })}
        />
      </div>

      {menu && (
        <ContextMenu x={menu.x} y={menu.y} onClose={() => setMenu(null)} items={[
          { label: "Open", action: () => void pathActions(menu.row.path, "open") },
          { label: "Open containing folder", action: () => void pathActions(menu.row.path, "folder") },
          { label: "Copy path", action: () => void pathActions(menu.row.path, "copy") },
          { label: "-" } as never,
          { label: "Recycle", danger: true, action: () => void pathActions(menu.row.path, "recycle") },
        ]} />
      )}
    </div>
  );
}
