/** Byte / date / number formatting shared across views (binary units). */

const UNITS = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  if (bytes < 1024) return `${Math.round(bytes)} B`;
  let v = bytes;
  let u = 0;
  while (v >= 1024 && u < UNITS.length - 1) {
    v /= 1024;
    u += 1;
  }
  if (v >= 100) return `${v.toFixed(0)} ${UNITS[u]}`;
  if (v >= 10) return `${v.toFixed(1)} ${UNITS[u]}`;
  return `${v.toFixed(2)} ${UNITS[u]}`;
}

export function formatCount(n: number): string {
  return n.toLocaleString("en-US");
}

export function formatPercent(p: number, digits = 1): string {
  return `${p.toFixed(digits)}%`;
}

export function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`;
  return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
}

export function formatTimer(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "00:00";
  const totalSec = Math.floor(ms / 1000);
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
}

export function formatETA(sec: number): string {
  if (!Number.isFinite(sec) || sec <= 0 || sec > 36000) return "evaluating…";
  if (sec < 60) return `~${Math.round(sec)}s remaining`;
  const m = Math.floor(sec / 60);
  const s = Math.round(sec % 60);
  return `~${m}m ${s}s remaining`;
}

export function formatDate(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

export function formatDateTime(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleString(undefined, {
    year: "numeric", month: "short", day: "numeric",
    hour: "2-digit", minute: "2-digit",
  });
}

export function ageFrom(iso: string | null): string {
  if (!iso) return "—";
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return "—";
  const days = Math.floor((Date.now() - then) / 86_400_000);
  if (days < 1) return "today";
  if (days === 1) return "yesterday";
  if (days < 30) return `${days} days ago`;
  if (days < 365) return `${Math.floor(days / 30)} months ago`;
  return `${(days / 365).toFixed(1)} years ago`;
}
