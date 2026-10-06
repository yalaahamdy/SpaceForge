/**
 * Storage Map — the interactive treemap (spec §13).
 *
 * Canvas rendering with squarified layout, zoom drill-down, breadcrumbs,
 * hover details, category coloring and a side legend. Large scans render
 * only the current level (children of one folder), which keeps the canvas
 * fast even for millions of files.
 */

import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ipc } from "../api/ipc";
import type { DirNode, FileEntry } from "../api/types";
import { CATEGORY_COLORS, colorFor, labelFor } from "../lib/categoryColor";
import { formatBytes, formatCount, ageFrom } from "../lib/format";
import { fitLabel, squarify, TreemapItem, TreemapRect } from "../lib/treemap";
import { EmptyState, ContextMenu, CtxMenuItem, usePathActions } from "../components/common/UI";
import { IconChevronLeft } from "../components/common/icons";
import { useApp } from "../state/AppState";

interface Hover {
  rect: TreemapRect;
  screenX: number;
  screenY: number;
}

export function TreemapView() {
  const { lastScan, scanning, startScan, selectedVolume, volumes, toast } = useApp();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const [tree, setTree] = useState<DirNode | null>(null);
  const [stack, setStack] = useState<string[]>([]); // path chain from root
  const [hover, setHover] = useState<Hover | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; items: CtxMenuItem[] } | null>(null);
  const [filter, setFilter] = useState("");
  const [minShare, setMinShare] = useState(0); // hide items smaller than N% of current level
  const [categoryFilter, setCategoryFilter] = useState<string>("all");
  const [size, setSize] = useState({ w: 800, h: 520 });
  const pathActions = usePathActions();

  // ---- load root / drill target -------------------------------------------
  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const path = stack.length ? stack[stack.length - 1] : "";
        const node = await ipc.drillDown(path);
        if (alive) setTree(node);
      } catch (e) {
        if (alive) setTree(null);
      }
    })();
    return () => { alive = false; };
  }, [stack, lastScan]);

  // ---- resize observer -------------------------------------------------------
  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      setSize({ w: el.clientWidth, h: el.clientHeight });
    });
    ro.observe(el);
    setSize({ w: el.clientWidth, h: el.clientHeight });
    return () => ro.disconnect();
  }, []);

  // ---- build items -----------------------------------------------------------
  const items = useMemo<TreemapItem[]>(() => {
    if (!tree) return [];
    const total = tree.size || 1;
    const share = minShare / 100;
    const dirs: TreemapItem[] = tree.children
      .filter((c) => c.size / total >= share)
      .map((c) => ({
        key: "d:" + c.path, name: c.name, size: c.size, kind: "dir",
        category: dominantCategory(c),
      }));
    const files: TreemapItem[] = tree.files
      .map((f) => ({
        key: "f:" + f.path, name: f.name, size: f.size, kind: "file",
        category: f.category,
      }));
    const smallAgg = tree.smallFilesSize;
    const all = [...dirs, ...files];
    const listed = all.reduce((a, b) => a + b.size, 0);
    if (smallAgg > 0) {
      all.push({
        key: "agg:small", name: `Many small files (${formatCount(tree.smallFilesCount)})`,
        size: smallAgg, kind: "aggregate", category: "unknown",
      });
    }
    const rest = Math.max(0, tree.size - listed - smallAgg);
    if (rest / total >= share && share > 0) {
      all.push({ key: "agg:hidden", name: "Filtered out", size: rest, kind: "aggregate", category: "unknown" });
    }
    const filtered = categoryFilter === "all"
      ? all
      : all.filter((i) => i.category === categoryFilter);
    if (filter.trim()) {
      const q = filter.toLowerCase();
      return filtered.filter((i) => i.name.toLowerCase().includes(q));
    }
    return filtered;
  }, [tree, filter, minShare, categoryFilter]);

  // ---- render ------------------------------------------------------------------
  const rects = useMemo(
    () => squarify(items, 0, 0, Math.max(size.w - 2, 10), Math.max(size.h - 2, 10)),
    [items, size],
  );

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = size.w * dpr;
    canvas.height = size.h * dpr;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, size.w, size.h);
    const gap = 2;
    for (const r of rects) {
      const x = r.x + gap / 2;
      const y = r.y + gap / 2;
      const w = Math.max(0, r.w - gap);
      const h = Math.max(0, r.h - gap);
      if (w < 0.5 || h < 0.5) continue;
      const base = CATEGORY_COLORS[(r.item.category ?? "unknown") as keyof typeof CATEGORY_COLORS]
        ?? CATEGORY_COLORS.unknown;
      ctx.fillStyle = base;
      ctx.globalAlpha = r.item.kind === "file" ? 0.86 : 0.8;
      ctx.beginPath();
      ctx.roundRect(x, y, w, h, 3);
      ctx.fill();
      ctx.globalAlpha = 1;
      if (r === hover?.rect) {
        ctx.strokeStyle = "#ffffff";
        ctx.lineWidth = 2;
        ctx.stroke();
      }
      // labels for large-enough rects
      if (w > 44 && h > 22) {
        ctx.fillStyle = "rgba(0,0,0,0.72)";
        ctx.font = "600 11.5px 'Segoe UI', system-ui, sans-serif";
        const name = fitLabel(ctx, r.item.name, w - 10);
        ctx.fillText(name, x + 5, y + 14);
        if (h > 38) {
          ctx.font = "10.5px 'Segoe UI', system-ui, sans-serif";
          ctx.fillStyle = "rgba(255,255,255,0.82)";
          ctx.fillText(formatBytes(r.item.size), x + 5, y + 28);
        }
      }
    }
  }, [rects, size, hover]);

  useEffect(() => { draw(); }, [draw]);

  const hit = (clientX: number, clientY: number): TreemapRect | null => {
    const canvas = canvasRef.current;
    if (!canvas) return null;
    const rect = canvas.getBoundingClientRect();
    const x = clientX - rect.left;
    const y = clientY - rect.top;
    for (const r of rects) {
      if (x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h) return r;
    }
    return null;
  };

  if (scanning) {
    return <EmptyState title="Scanning…" hint="The storage map appears as soon as the scan finishes." />;
  }
  if (!lastScan) {
    return (
      <div>
        <h1 className="page-title">Storage Map</h1>
        <p className="page-sub">Scan a drive to explore it as a visual map.</p>
        <EmptyState
          title="No scan available"
          hint="Run a scan from the Dashboard or press the button below — the map renders instantly afterwards."
          action={
            <button className="btn primary" onClick={() => startScan(selectedVolume)}>
              Scan {selectedVolume || "system drive"}
            </button>
          }
        />
      </div>
    );
  }

  const crumbs: { label: string; path: string }[] = [
    { label: lastScan.volume, path: "" },
    ...stack.map((p, i) => ({
      label: p.split(/[\\/]/).filter(Boolean).pop() ?? p,
      path: p,
    })),
  ];

  const total = tree?.size ?? 1;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "calc(100vh - 120px)" }}>
      <div className="row-wrap" style={{ marginBottom: 10 }}>
        <h1 className="page-title" style={{ margin: 0 }}>Storage Map</h1>
        <div className="spacer" />
        <div className="row" style={{ gap: 8 }}>
          <label className="dim" htmlFor="sf-map-search">Find</label>
          <input id="sf-map-search" type="text" placeholder="name contains…" value={filter}
            onChange={(e) => setFilter(e.target.value)} style={{ width: 170 }} />
          <select value={categoryFilter} onChange={(e) => setCategoryFilter(e.target.value)}
            aria-label="Filter by category">
            <option value="all">All categories</option>
            {Object.keys(CATEGORY_COLORS).map((c) => (
              <option key={c} value={c}>{labelFor(c)}</option>
            ))}
          </select>
          <label className="dim" htmlFor="sf-map-share">Hide &lt;</label>
          <input id="sf-map-share" type="number" min={0} max={90} value={minShare}
            onChange={(e) => setMinShare(Math.max(0, Math.min(90, Number(e.target.value) || 0)))}
            style={{ width: 64 }} aria-label="Hide items smaller than this percentage" />
          <span className="dim">%</span>
        </div>
      </div>

      <div className="breadcrumbs" style={{ marginBottom: 8 }}>
        {crumbs.map((c, i) => (
          <React.Fragment key={c.path + i}>
            {i > 0 && <span className="sep">›</span>}
            <button onClick={() => setStack((s) => s.slice(0, i))}>{c.label}</button>
          </React.Fragment>
        ))}
        {stack.length > 0 && (
          <button onClick={() => setStack((s) => s.slice(0, -1))} aria-label="Go up one level">
            <IconChevronLeft size={13} />
          </button>
        )}
      </div>

      <div ref={wrapRef} style={{ flex: 1, minHeight: 320, position: "relative" }}>
        <canvas
          ref={canvasRef}
          style={{ width: "100%", height: "100%", borderRadius: 10, cursor: hover ? "pointer" : "default" }}
          onMouseMove={(e) => {
            const r = hit(e.clientX, e.clientY);
            setHover(r ? { rect: r, screenX: e.clientX, screenY: e.clientY } : null);
          }}
          onMouseLeave={() => setHover(null)}
          onClick={(e) => {
            const r = hit(e.clientX, e.clientY);
            if (r?.item.kind === "dir") {
              const path = r.item.key.slice(2);
              setStack((s) => [...s, path]);
            }
          }}
          onContextMenu={(e) => {
            const r = hit(e.clientX, e.clientY);
            if (!r) return;
            e.preventDefault();
            const path = r.item.key.startsWith("f:") || r.item.key.startsWith("d:")
              ? r.item.key.slice(2)
              : null;
            const items2: CtxMenuItem[] = [];
            if (path) {
              items2.push(
                { label: "Open containing folder", action: () => void pathActions(path, "folder") },
                { label: "Copy path", action: () => void pathActions(path, "copy") },
                { label: "-" as unknown as string, action: () => undefined } as CtxMenuItem,
                { label: "Recycle", danger: true, action: () => void pathActions(path, "recycle") },
              );
            }
            if (r.item.kind === "dir") {
              items2.push({ label: "Zoom into folder", action: () => setStack((s) => [...s, r.item.key.slice(2)]) });
            }
            setMenu({ x: e.clientX, y: e.clientY, items: items2 });
          }}
          aria-label="Treemap of scanned storage. Click a folder rectangle to zoom in."
        />
        {hover && (
          <div
            style={{
              position: "fixed",
              left: Math.min(hover.screenX + 14, window.innerWidth - 280),
              top: hover.screenY + 14,
              background: "var(--bg-s2)",
              border: "1px solid var(--stroke-2)",
              borderRadius: 8,
              padding: "8px 12px",
              pointerEvents: "none",
              boxShadow: "var(--shadow-2)",
              zIndex: 500,
              maxWidth: 320,
            }}
          >
            <div style={{ fontWeight: 650, fontSize: 13, wordBreak: "break-all" }}>{hover.rect.item.name}</div>
            <div className="dim" style={{ fontSize: 12, marginTop: 2 }}>
              {formatBytes(hover.rect.item.size)} · {(100 * hover.rect.item.size / total).toFixed(1)}% of{" "}
              {tree?.name || lastScan.volume}
            </div>
            {hover.rect.item.kind === "dir" && (
              <div className="dim3" style={{ fontSize: 11.5, marginTop: 2 }}>Click to zoom in</div>
            )}
          </div>
        )}
        {menu && (
          <ContextMenu x={menu.x} y={menu.y} items={menu.items} onClose={() => setMenu(null)} />
        )}
      </div>

      <div className="row" style={{ marginTop: 10, gap: 14, flexWrap: "wrap" }}>
        {(["video", "image", "document", "archive", "disk_image", "game", "system", "cache", "developer", "unknown"] as const).map((c) => (
          <span key={c} className="row" style={{ gap: 5, fontSize: 11.5, color: "var(--text-2)" }}>
            <span style={{ width: 9, height: 9, borderRadius: 3, background: colorFor(c) }} />
            {labelFor(c)}
          </span>
        ))}
      </div>
    </div>
  );
}

/** Dominant category of a subtree, by size (cheap: uses indexed files). */
function dominantCategory(node: DirNode): string {
  const tally = new Map<string, number>();
  for (const f of node.files) {
    tally.set(f.category, (tally.get(f.category) ?? 0) + f.size);
  }
  for (const c of node.children) {
    const cat = dominantCategory(c);
    if (cat) tally.set(cat, (tally.get(cat) ?? 0) + 1);
  }
  let best = "";
  let bestV = -1;
  for (const [k, v] of tally) {
    if (v > bestV) { best = k; bestV = v; }
  }
  return best || "unknown";
}

export type { FileEntry };
