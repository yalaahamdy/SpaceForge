/** Shell: side navigation, title bar, status bar. */

import React, { useEffect, useState } from "react";
import { useApp, ViewId } from "../../state/AppState";
import { formatBytes } from "../../lib/format";
import {
  IconActivity, IconAdvanced, IconAnalyzer, IconApps, IconCleanup, IconClock,
  IconCode, IconDashboard, IconDuplicates, IconFileTypes, IconGlobe,
  IconHistory, IconInsights, IconLargeFiles, IconQuestion, IconSettings,
  IconTreemap, IconWindows,
} from "../common/icons";

interface NavEntry {
  id: ViewId;
  label: string;
  icon: React.ComponentType<{ size?: number }>;
}

const NAV: { group: string; items: NavEntry[] }[] = [
  {
    group: "Discover",
    items: [
      { id: "dashboard", label: "Dashboard", icon: IconDashboard },
      { id: "analyzer", label: "Disk Analyzer", icon: IconAnalyzer },
      { id: "treemap", label: "Storage Map", icon: IconTreemap },
      { id: "large-files", label: "Large Files", icon: IconLargeFiles },
      { id: "file-types", label: "File Types", icon: IconFileTypes },
      { id: "duplicates", label: "Duplicates", icon: IconDuplicates },
    ],
  },
  {
    group: "Clean",
    items: [
      { id: "cleanup-center", label: "Cleanup Center", icon: IconCleanup },
      { id: "windows-cleanup", label: "Windows Cleanup", icon: IconWindows },
      { id: "app-cleanup", label: "Applications", icon: IconApps },
      { id: "browser-cleanup", label: "Browsers", icon: IconGlobe },
      { id: "dev-cleanup", label: "Developer Caches", icon: IconCode },
    ],
  },
  {
    group: "Monitor",
    items: [
      { id: "history", label: "Storage History", icon: IconHistory },
      { id: "insights", label: "Space Insights", icon: IconInsights },
      { id: "activity", label: "Cleanup History", icon: IconActivity },
    ],
  },
  {
    group: "Advanced",
    items: [
      { id: "old-files", label: "Old Files", icon: IconClock },
      { id: "why-full", label: "Why Is My Disk Full?", icon: IconQuestion },
      { id: "advanced", label: "Advanced Tools", icon: IconAdvanced },
      { id: "settings", label: "Settings", icon: IconSettings },
    ],
  },
];

export function SideNav() {
  const { view, navigate } = useApp();
  return (
    <nav className="sidenav" aria-label="Primary">
      {NAV.map((g) => (
        <div key={g.group} role="group">
          <div className="nav-group-label">{g.group}</div>
          {g.items.map((it) => (
            <button
              key={it.id}
              className={`nav-item${view === it.id ? " active" : ""}`}
              onClick={() => navigate(it.id)}
              aria-current={view === it.id ? "page" : undefined}
            >
              <it.icon size={16} />
              {it.label}
            </button>
          ))}
        </div>
      ))}
    </nav>
  );
}

export function TitleBar() {
  const { theme, toggleTheme, settings } = useApp();
  return (
    <header className="titlebar">
      <div className="brand">
        <img src="/icon.svg" alt="" />
        SpaceForge
        <span className="dim3" style={{ fontSize: 11 }}>v1.0.0</span>
      </div>

      <div className="spacer" />

      <button
        className="cosmic-chip"
        style={{ cursor: "pointer", background: "var(--bg-s3)", border: "1px solid var(--stroke-2)" }}
        onClick={() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", ctrlKey: true }))}
        title="Open Command Palette (Ctrl+K)"
      >
        <kbd style={{ fontSize: 10, padding: "0 4px" }}>Ctrl+K</kbd>
        <span style={{ fontSize: 11 }}>Quick Jump</span>
      </button>

      <span className="dim3" style={{ fontSize: 11.5 }}>
        telemetry: {settings.telemetry ? "on" : "off · local-only"}
      </span>

      <button
        className="btn ghost small"
        onClick={toggleTheme}
        style={{ gap: 6 }}
        aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}
      >
        <span
          className="chip-dot"
          style={{ background: theme === "dark" ? "var(--accent)" : "var(--accent-2)" }}
        />
        {theme === "dark" ? "Dark Theme" : "Light Theme"}
      </button>
    </header>
  );
}

export function StatusBar() {
  const { scanning, scanProgress, lastScan, volumes, selectedVolume, elevated } = useApp();
  const [clock, setClock] = useState(new Date());
  useEffect(() => {
    const t = window.setInterval(() => setClock(new Date()), 30_000);
    return () => window.clearInterval(t);
  }, []);
  const vol = volumes.find((v) => v.mountPoint === selectedVolume);
  return (
    <footer className="statusbar">
      {scanning ? (
        <span className="row" style={{ gap: 8 }}>
          <span className="scan-pulse" />
          <span style={{ fontWeight: 600 }}>Scanning {scanProgress?.volume}</span>
          <span className="num" style={{ color: "var(--accent)", fontWeight: 700 }}>
            {scanProgress && scanProgress.elapsedMs > 400
              ? `${formatBytes(scanProgress.bytesSeen / (scanProgress.elapsedMs / 1000))}/s`
              : "initializing…"}
          </span>
          <span className="dim3">·</span>
          <span className="num">{(scanProgress?.filesSeen ?? 0).toLocaleString()} files</span>
          <span className="dim3">·</span>
          <span className="num">{formatBytes(scanProgress?.bytesSeen ?? 0)}</span>
        </span>
      ) : (
        <span>
          {lastScan
            ? `Last scan: ${lastScan.volume} · ${new Date(lastScan.startedAt).toLocaleString()}`
            : "No scan yet"}
        </span>
      )}
      <div className="spacer" />
      {vol && (
        <span>
          {vol.mountPoint} {formatBytes(vol.freeBytes)} free of {formatBytes(vol.totalBytes)}
        </span>
      )}
      <span>{elevated ? "administrator" : "standard user"}</span>
      <span>{clock.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}</span>
    </footer>
  );
}
