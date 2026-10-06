/** Shared UI primitives: badges, donut chart, empty states, dialogs,
 *  context menu, virtual table, toasts, volume picker. */

import React, { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { RISK_COLORS, RISK_LABELS, colorFor, labelFor } from "../../lib/categoryColor";
import { formatBytes, formatCount } from "../../lib/format";
import { useApp } from "../../state/AppState";
import { IconClose, IconFolder, IconSearch } from "./icons";

// ---------------------------------------------------------------------------
// RiskBadge

export function RiskBadge({ risk }: { risk: string }) {
  return (
    <span className={`badge ${risk}`} style={{ borderColor: RISK_COLORS[risk] + "55" }}>
      {RISK_LABELS[risk] ?? risk}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Donut chart (canvas, no deps)

export function Donut({
  data, size = 190, thickness = 26, onSlice,
}: {
  data: { label: string; value: number; color: string }[];
  size?: number;
  thickness?: number;
  onSlice?: (label: string | null) => void;
}) {
  const ref = useRef<HTMLCanvasElement>(null);
  const total = data.reduce((a, b) => a + b.value, 0);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = size * dpr;
    canvas.height = size * dpr;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, size, size);
    if (total <= 0) return;
    const cx = size / 2;
    const cy = size / 2;
    const r = size / 2 - thickness / 2 - 2;
    let start = -Math.PI / 2;
    for (const d of data) {
      if (d.value <= 0) continue;
      const angle = (d.value / total) * Math.PI * 2;
      ctx.beginPath();
      ctx.arc(cx, cy, r, start, start + angle);
      ctx.strokeStyle = d.color;
      ctx.lineWidth = thickness;
      ctx.lineCap = "butt";
      ctx.stroke();
      start += angle;
    }
    // center hole
    ctx.globalCompositeOperation = "destination-out";
    ctx.beginPath();
    ctx.arc(cx, cy, r - thickness / 2 - 1, 0, Math.PI * 2);
    ctx.fill();
    ctx.globalCompositeOperation = "source-over";
  }, [data, size, thickness, total]);

  return (
    <canvas
      ref={ref}
      style={{ width: size, height: size }}
      role="img"
      aria-label="Storage breakdown"
      onMouseMove={(e) => {
        if (!onSlice) return;
        const rect = e.currentTarget.getBoundingClientRect();
        const dx = e.clientX - rect.left - size / 2;
        const dy = e.clientY - rect.top - size / 2;
        const dist = Math.hypot(dx, dy);
        const r = size / 2 - thickness / 2 - 2;
        if (dist < r - thickness || dist > r + thickness / 2) return onSlice(null);
        let a = Math.atan2(dy, dx) + Math.PI / 2;
        if (a < 0) a += Math.PI * 2;
        let acc = 0;
        for (const d of data) {
          const angle = (d.value / total) * Math.PI * 2;
          if (a >= acc && a < acc + angle) return onSlice(d.label);
          acc += angle;
        }
        onSlice(null);
      }}
      onMouseLeave={() => onSlice?.(null)}
    />
  );
}

// ---------------------------------------------------------------------------
// Sparkline (storage history)

export function Sparkline({
  points, width = 560, height = 140,
}: {
  points: { at: string; used: number }[];
  width?: number;
  height?: number;
}) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, width, height);
    if (points.length < 2) return;
    const vals = points.map((p) => p.used);
    const min = Math.min(...vals);
    const max = Math.max(...vals);
    const span = Math.max(max - min, 1);
    const stepX = (width - 20) / (points.length - 1);
    const toXY = (v: number, i: number): [number, number] => [
      10 + i * stepX,
      height - 20 - ((v - min) / span) * (height - 40),
    ];
    // area
    ctx.beginPath();
    ctx.moveTo(...toXY(vals[0], 0));
    vals.forEach((v, i) => ctx.lineTo(...toXY(v, i)));
    ctx.lineTo(...toXY(vals[vals.length - 1], vals.length - 1));
    ctx.lineTo(10 + (points.length - 1) * stepX, height - 18);
    ctx.lineTo(10, height - 18);
    ctx.closePath();
    ctx.fillStyle = "rgba(59,130,246,0.14)";
    ctx.fill();
    // line
    ctx.beginPath();
    ctx.moveTo(...toXY(vals[0], 0));
    vals.forEach((v, i) => ctx.lineTo(...toXY(v, i)));
    ctx.strokeStyle = "#3b82f6";
    ctx.lineWidth = 2;
    ctx.stroke();
    // last dot
    const [lx, ly] = toXY(vals[vals.length - 1], vals.length - 1);
    ctx.beginPath();
    ctx.arc(lx, ly, 3.5, 0, Math.PI * 2);
    ctx.fillStyle = "#f59e0b";
    ctx.fill();
  }, [points, width, height]);
  return <canvas ref={ref} style={{ width, height, maxWidth: "100%" }} />;
}

// ---------------------------------------------------------------------------
// Empty state

export function EmptyState({
  icon, title, hint, action,
}: {
  icon?: React.ReactNode;
  title: string;
  hint?: string;
  action?: React.ReactNode;
}) {
  return (
    <div className="empty">
      {icon ?? <IconSearch size={38} />}
      <div className="title">{title}</div>
      {hint && <div className="hint">{hint}</div>}
      {action}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Modal

export function Modal({
  title, children, footer, onClose,
}: {
  title: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
  onClose: () => void;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);
  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal" role="dialog" aria-modal="true" aria-label={title}>
        <div className="modal-head row">
          {title}
          <div className="spacer" />
          <button className="btn ghost small" onClick={onClose} aria-label="Close">
            <IconClose size={14} />
          </button>
        </div>
        <div className="modal-body">{children}</div>
        {footer && <div className="modal-foot">{footer}</div>}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Context menu

export interface CtxMenuItem {
  label: string;
  danger?: boolean;
  action: () => void;
}

export function ContextMenu({
  x, y, items, onClose,
}: {
  x: number;
  y: number;
  items: CtxMenuItem[];
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const esc = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", esc);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", esc);
    };
  }, [onClose]);
  const [pos, setPos] = useState({ x, y });
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    setPos({
      x: Math.min(x, window.innerWidth - r.width - 8),
      y: Math.min(y, window.innerHeight - r.height - 8),
    });
  }, [x, y]);
  return (
    <div className="ctx-menu" ref={ref} style={{ left: pos.x, top: pos.y }}>
      {items.map((it, i) =>
        it.label === "-" ? (
          <div className="divider" key={i} />
        ) : (
          <button key={i} className={it.danger ? "danger" : ""} onClick={() => { it.action(); onClose(); }}>
            {it.label}
          </button>
        ),
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Virtual table (windowed rows — handles 100k+ rows smoothly)

export interface Column<T> {
  key: string;
  label: string;
  width: string;
  align?: "left" | "right";
  render: (row: T) => React.ReactNode;
  sortValue?: (row: T) => number | string;
}

export function VirtualTable<T>({
  rows, columns, rowHeight = 34, defaultSort, onRowContext, onRowActivate,
}: {
  rows: T[];
  columns: Column<T>[];
  rowHeight?: number;
  defaultSort?: string;
  onRowContext?: (row: T, x: number, y: number) => void;
  onRowActivate?: (row: T) => void;
}) {
  const [sortKey, setSortKey] = useState<string | null>(defaultSort ?? null);
  const [dir, setDir] = useState<1 | -1>(-1);
  const [scrollTop, setScrollTop] = useState(0);
  const [height, setHeight] = useState(420);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setHeight(el.clientHeight));
    ro.observe(el);
    setHeight(el.clientHeight);
    return () => ro.disconnect();
  }, []);

  const sorted = React.useMemo(() => {
    if (!sortKey) return rows;
    const col = columns.find((c) => c.key === sortKey);
    if (!col?.sortValue) return rows;
    return [...rows].sort((a, b) => {
      const va = col.sortValue!(a);
      const vb = col.sortValue!(b);
      if (va < vb) return -1 * dir;
      if (va > vb) return 1 * dir;
      return 0;
    });
  }, [rows, columns, sortKey, dir]);

  const gridTemplate = columns.map((c) => c.width).join(" ");
  const total = sorted.length;
  const viewportRows = Math.ceil(height / rowHeight);
  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - 4);
  const end = Math.min(total, start + viewportRows + 8);
  const visible = sorted.slice(start, end);

  const onSort = useCallback((key: string) => {
    const col = columns.find((c) => c.key === key);
    if (!col?.sortValue) return;
    setSortKey((k) => (k === key ? (dir === 1 ? key : key) : key));
    setDir((d) => (sortKey === key ? ((d * -1) as 1 | -1) : -1));
  }, [columns, dir, sortKey]);

  return (
    <div className="vtable" style={{ height: "100%", minHeight: 260 }} ref={containerRef}>
      <div className="vtable-head" style={{ display: "grid", gridTemplateColumns: gridTemplate }}>
        {columns.map((c) => (
          <div key={c.key} className="hcell" style={{ textAlign: c.align ?? "left" }}
            onClick={() => onSort(c.key)}>
            {c.label}
            {sortKey === c.key ? (dir === 1 ? " ▲" : " ▼") : ""}
          </div>
        ))}
      </div>
      <div
        style={{ height: `calc(100% - 33px)`, overflowY: "auto", position: "relative" }}
        onScroll={(e) => setScrollTop((e.target as HTMLDivElement).scrollTop)}
      >
        <div style={{ height: total * rowHeight, position: "relative" }}>
          <div style={{ position: "absolute", top: start * rowHeight, left: 0, right: 0 }}>
            {visible.map((row, i) => (
              <div
                key={start + i}
                className="vtable-row"
                style={{ gridTemplateColumns: gridTemplate, height: rowHeight }}
                onContextMenu={(e) => {
                  if (onRowContext) {
                    e.preventDefault();
                    onRowContext(row, e.clientX, e.clientY);
                  }
                }}
                onDoubleClick={() => onRowActivate?.(row)}
              >
                {columns.map((c) => (
                  <div key={c.key} className={`cell ${c.align === "right" ? "num" : ""}`}>
                    {c.render(row)}
                  </div>
                ))}
              </div>
            ))}
          </div>
        </div>
        {total === 0 && (
          <div className="dim3" style={{ padding: 18, fontSize: 13 }}>No rows to display.</div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Toasts

export function ToastHost() {
  const { toasts } = useApp();
  return (
    <div className="toast-host">
      {toasts.map((t) => (
        <div key={t.id} className={`toast ${t.kind}`} role="status">
          {t.text}
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Category legend + volume picker + file context menu helper

export function CategoryLegend({
  data,
}: {
  data: [string, number, number][];
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {data.map(([cat, size, count]) => (
        <div key={cat} className="row" style={{ gap: 8, fontSize: 12.5 }}>
          <span style={{ width: 10, height: 10, borderRadius: 3, background: colorFor(cat), flexShrink: 0 }} />
          <span className="dim">{labelFor(cat)}</span>
          <div className="spacer" />
          <span className="num">{formatBytes(size)}</span>
          <span className="dim3 num" style={{ width: 68, textAlign: "right" }}>
            {formatCount(count)} files
          </span>
        </div>
      ))}
    </div>
  );
}

/** Actions for a scanned file/folder path (used by many views). */
export function usePathActions() {
  const { toast } = useApp();
  return useCallback(
    async (path: string, action: "open" | "folder" | "copy" | "recycle" | "delete") => {
      try {
        switch (action) {
          case "open":
            await ipc_open(path);
            break;
          case "folder":
            await ipcFolder(path);
            break;
          case "copy":
            await navigator.clipboard.writeText(path).catch(() => undefined);
            toast("Path copied to clipboard", "ok");
            break;
          case "recycle": {
            const r = await ipcRecycle([path]);
            toast(`Moved to Recycle Bin (${r.recycled})`, "ok");
            break;
          }
          case "delete": {
            const r = await ipcDelete([path]);
            toast(`Deleted ${r.deleted}, skipped ${r.skipped}, failed ${r.failures.length}`, r.failures.length ? "error" : "ok");
            break;
          }
        }
      } catch (e) {
        toast(String(e), "error");
      }
    },
    [toast],
  );
}

import { ipc } from "../../api/ipc";
async function ipc_open(path: string) { await ipc.openPath(path); }
async function ipcFolder(path: string) { await ipc.openInExplorer(path, true); }
async function ipcRecycle(paths: string[]) { return ipc.recyclePaths(paths); }
async function ipcDelete(paths: string[]) { return ipc.deletePaths(paths); }

export { IconFolder };
