/**
 * Squarified treemap layout (Bruls, Huizing & van Wijk) — the algorithm
 * behind the Storage Map (spec §13). Pure functions, canvas-rendered.
 */

export interface TreemapItem {
  key: string;
  name: string;
  size: number;
  category?: string;
  kind: "dir" | "file" | "aggregate";
}

export interface TreemapRect {
  item: TreemapItem;
  x: number;
  y: number;
  w: number;
  h: number;
}

/**
 * Lay items into a rectangle. Items must be pre-sorted by size desc.
 * Areas are proportional to `size`; zero-size items are skipped.
 */
export function squarify(
  items: TreemapItem[],
  x: number,
  y: number,
  w: number,
  h: number,
): TreemapRect[] {
  const total = items.reduce((a, b) => a + b.size, 0);
  if (total <= 0 || w <= 0 || h <= 0) return [];
  const sorted = [...items].filter((i) => i.size > 0).sort((a, b) => b.size - a.size);
  const out: TreemapRect[] = [];
  layoutRow(sorted, x, y, w, h, total, out);
  return out;
}

function layoutRow(
  items: TreemapItem[],
  x: number,
  y: number,
  w: number,
  h: number,
  totalArea: number,
  out: TreemapRect[],
): void {
  if (items.length === 0) return;
  const scale = (w * h) / totalArea;
  const sized = items.map((it) => ({ item: it, area: it.size * scale }));

  let remaining = sized;
  let rx = x;
  let ry = y;
  let rw = w;
  let rh = h;

  while (remaining.length > 0) {
    const row = pickRow(remaining, Math.min(rw, rh));
    const rowArea = row.reduce((a, b) => a + b.area, 0);
    const horiz = rw >= rh; // row along the longer side
    const thickness = rowArea / (horiz ? rh : rw);

    let off = 0;
    for (const cell of row) {
      const frac = cell.area / rowArea;
      if (horiz) {
        out.push({ item: cell.item, x: rx, y: ry + off, w: thickness, h: frac * rh });
      } else {
        out.push({ item: cell.item, x: rx + off, y: ry, w: frac * rw, h: thickness });
      }
      off += frac * (horiz ? rh : rw);
    }

    if (horiz) {
      rx += thickness;
      rw -= thickness;
    } else {
      ry += thickness;
      rh -= thickness;
    }
    if (rw <= 0.5 || rh <= 0.5) {
      // Give any leftovers a sliver so nothing visually disappears.
      if (rw > 0.05 || rh > 0.05) {
        for (const cell of remaining.filter((r) => !row.includes(r))) {
          out.push({ item: cell.item, x: rx, y: ry, w: Math.max(rw, 0.5), h: Math.max(rh, 0.5) });
        }
      }
      return;
    }
    remaining = remaining.filter((r) => !row.includes(r));
    if (remaining.length === 1) {
      const last = remaining[0];
      out.push({ item: last.item, x: rx, y: ry, w: rw, h: rh });
      return;
    }
  }
}

/** Worst-ratio row picking (squarify core). */
function pickRow(
  candidates: { item: TreemapItem; area: number }[],
  side: number,
): { item: TreemapItem; area: number }[] {
  const row: { item: TreemapItem; area: number }[] = [];
  let rowArea = 0;
  let bestRatio = Infinity;
  for (const c of candidates) {
    const newArea = rowArea + c.area;
    const newRatio = worstRatio([...row, c], newArea, side);
    if (row.length === 0 || newRatio <= bestRatio) {
      row.push(c);
      rowArea = newArea;
      bestRatio = newRatio;
    } else {
      break;
    }
  }
  return row;
}

function worstRatio(
  row: { area: number }[],
  rowArea: number,
  side: number,
): number {
  const thickness = rowArea / side;
  let worst = 0;
  for (const cell of row) {
    const len = cell.area / Math.max(thickness, 1e-9);
    const ratio = Math.max(len / Math.max(thickness, 1e-9), thickness / Math.max(len, 1e-9));
    worst = Math.max(worst, ratio);
  }
  return worst;
}

/** Human label clipped to a rect. */
export function fitLabel(ctx: CanvasRenderingContext2D, label: string, maxW: number): string {
  if (ctx.measureText(label).width <= maxW) return label;
  let s = label;
  while (s.length > 1 && ctx.measureText(s + "…").width > maxW) {
    s = s.slice(0, -1);
  }
  return s + "…";
}
