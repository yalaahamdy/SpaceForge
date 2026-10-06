/** Application shell: nav + view routing + global search + toasts. */

import React, { useEffect, useState } from "react";
import { AppStateProvider, useApp } from "./state/AppState";
import { SideNav, StatusBar, TitleBar } from "./components/shell/Shell";
import { ToastHost } from "./components/common/UI";
import { DashboardView } from "./views/DashboardView";
import { AnalyzerView } from "./views/AnalyzerView";
import { TreemapView } from "./views/TreemapView";
import { LargeFilesView } from "./views/LargeFilesView";
import { DuplicatesView, FileTypesView } from "./views/FileTypesDuplicates";
import { CleanupView } from "./views/CleanupViews";
import { ActivityView, HistoryView, InsightsView, OldFilesView, WhyFullView } from "./views/InfoViews";
import { AdvancedView, SearchPanel, SettingsView } from "./views/AdvancedSettings";
import { ipc } from "./api/ipc";
import { IconSearch } from "./components/common/icons";

function ViewRouter() {
  const { view } = useApp();
  const renderView = () => {
    switch (view) {
      case "dashboard": return <DashboardView />;
      case "analyzer": return <AnalyzerView />;
      case "treemap": return <TreemapView />;
      case "large-files": return <LargeFilesView />;
      case "file-types": return <FileTypesView />;
      case "duplicates": return <DuplicatesView />;
      case "cleanup-center": return <CleanupView family="all" />;
      case "windows-cleanup": return <CleanupView family="windows" />;
      case "app-cleanup": return <CleanupView family="application" />;
      case "browser-cleanup": return <CleanupView family="browser" />;
      case "dev-cleanup": return <CleanupView family="developer" />;
      case "old-files": return <OldFilesView />;
      case "history": return <HistoryView />;
      case "insights": return <InsightsView />;
      case "why-full": return <WhyFullView />;
      case "advanced": return <AdvancedView />;
      case "activity": return <ActivityView />;
      case "settings": return <SettingsView />;
      default: return <DashboardView />;
    }
  };

  return (
    <div key={view} className="view-container">
      {renderView()}
    </div>
  );
}

/** Keyboard command palette: Ctrl+K jumps between areas (spec §65). */
function CommandPalette() {
  const { navigate, view } = useApp();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen((o) => !o);
      }
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const destinations: { id: Parameters<typeof navigate>[0]; label: string }[] = [
    { id: "dashboard", label: "Dashboard" },
    { id: "treemap", label: "Storage Map" },
    { id: "analyzer", label: "Disk Analyzer" },
    { id: "large-files", label: "Large Files" },
    { id: "duplicates", label: "Duplicates" },
    { id: "cleanup-center", label: "Cleanup Center" },
    { id: "windows-cleanup", label: "Windows Cleanup" },
    { id: "why-full", label: "Why Is My Disk Full?" },
    { id: "history", label: "Storage History" },
    { id: "advanced", label: "Advanced Tools" },
    { id: "settings", label: "Settings" },
  ];
  const matches = destinations.filter((d) =>
    d.label.toLowerCase().includes(query.toLowerCase()),
  );

  if (!open) return null;
  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && setOpen(false)}>
      <div className="modal" style={{ maxHeight: 380 }}>
        <div className="modal-body" style={{ paddingTop: 14 }}>
          <input
            autoFocus
            type="text"
            placeholder="Go to… (Ctrl+K)"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            style={{ width: "100%" }}
            aria-label="Navigate to view"
          />
          <div style={{ marginTop: 10 }}>
            {matches.map((d) => (
              <button
                key={d.id}
                className={`btn ghost ${view === d.id ? "" : ""}`}
                style={{ width: "100%", justifyContent: "flex-start" }}
                onClick={() => { navigate(d.id); setOpen(false); setQuery(""); }}
              >
                {d.label}
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

/** Global search card on the analyzer view (spec §43). */
function GlobalSearch() {
  const { view } = useApp();
  const [query, setQuery] = useState("");
  const [enabled, setEnabled] = useState(false);
  useEffect(() => {
    if (!enabled) {
      // Prime the search index availability (cheap status ping).
      void ipc.scanStatus().catch(() => undefined);
    }
  }, [enabled]);
  if (view !== "analyzer") return null;
  return (
    <div className="card" style={{ marginBottom: 12 }}>
      <div className="row">
        <IconSearch size={15} />
        <input
          type="text"
          placeholder="Search the scan index by name, extension or path…"
          value={query}
          onChange={(e) => { setQuery(e.target.value); setEnabled(true); }}
          style={{ flex: 1 }}
          aria-label="Search analyzed storage"
        />
      </div>
      {enabled && <SearchPanel externalQuery={query} />}
    </div>
  );
}

function AppInner() {
  return (
    <div className="app">
      <TitleBar />
      <SideNav />
      <main className="main">
        <GlobalSearch />
        <ViewRouter />
      </main>
      <StatusBar />
      <ToastHost />
      <CommandPalette />
    </div>
  );
}

export default function App() {
  return (
    <AppStateProvider>
      <AppInner />
    </AppStateProvider>
  );
}
