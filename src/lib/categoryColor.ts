/** Category color system for treemap, donut and badges (spec §13). */

import type { FileCategory } from "../api/types";

export const CATEGORY_COLORS: Record<FileCategory, string> = {
  video: "#c026d3",
  audio: "#a855f7",
  image: "#ec4899",
  document: "#f59e0b",
  archive: "#eab308",
  disk_image: "#f97316",
  executable: "#10b981",
  library: "#14b8a6",
  code: "#22d3ee",
  game: "#3b82f6",
  system: "#64748b",
  cache: "#fbbf24",
  temporary: "#fbbf24",
  log: "#94a3b8",
  database: "#8b5cf6",
  developer: "#06b6d4",
  unknown: "#4b5563",
};

export const CATEGORY_LABELS: Record<FileCategory, string> = {
  video: "Videos",
  audio: "Audio",
  image: "Images",
  document: "Documents",
  archive: "Archives",
  disk_image: "Disk images",
  executable: "Executables",
  library: "Libraries",
  code: "Code",
  game: "Games",
  system: "System",
  cache: "Cache",
  temporary: "Temporary",
  log: "Logs",
  database: "Databases",
  developer: "Developer data",
  unknown: "Unknown",
};

export function colorFor(cat: string): string {
  return CATEGORY_COLORS[cat as FileCategory] ?? CATEGORY_COLORS.unknown;
}

export function labelFor(cat: string): string {
  return CATEGORY_LABELS[cat as FileCategory] ?? "Unknown";
}

/** Risk badge colors. */
export const RISK_COLORS: Record<string, string> = {
  safe: "#10b981",
  low: "#22d3ee",
  review: "#f59e0b",
  advanced: "#f97316",
  dangerous: "#ef4444",
};

export const RISK_LABELS: Record<string, string> = {
  safe: "Safe",
  low: "Low risk",
  review: "Review",
  advanced: "Advanced",
  dangerous: "Dangerous",
};
