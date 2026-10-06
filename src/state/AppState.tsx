/**
 * Global application state: navigation, theme, scan lifecycle, toasts,
 * settings and the shared cleanup-opportunity cache.
 */

import React, {
  createContext, useCallback, useContext, useEffect, useMemo, useRef, useState,
} from "react";
import { ipc, listen } from "../api/ipc";
import {
  AppSettings, CleanupOpportunity, DEFAULT_SETTINGS, ScanProgress, ScanSummary,
  VolumeInfo,
} from "../api/types";

export type ViewId =
  | "dashboard" | "analyzer" | "treemap" | "large-files" | "file-types"
  | "duplicates" | "cleanup-center" | "windows-cleanup" | "app-cleanup"
  | "browser-cleanup" | "dev-cleanup" | "old-files" | "history"
  | "insights" | "why-full" | "advanced" | "activity" | "settings";

export interface Toast {
  id: number;
  text: string;
  kind: "info" | "ok" | "error";
}

interface AppCtx {
  view: ViewId;
  navigate: (v: ViewId) => void;
  theme: "dark" | "light";
  toggleTheme: () => void;
  volumes: VolumeInfo[];
  refreshVolumes: () => Promise<void>;
  selectedVolume: string;
  setSelectedVolume: (v: string) => void;
  scanning: boolean;
  scanProgress: ScanProgress | null;
  lastScan: ScanSummary | null;
  startScan: (volume: string) => Promise<void>;
  cancelScan: () => void;
  invalidateScan: () => void;
  settings: AppSettings;
  saveSettings: (s: AppSettings) => Promise<void>;
  toasts: Toast[];
  toast: (text: string, kind?: Toast["kind"]) => void;
  dismissToast: (id: number) => void;
  opportunities: CleanupOpportunity[];
  setOpportunities: (o: CleanupOpportunity[]) => void;
  elevated: boolean;
  refreshElevation: () => Promise<void>;
}

const Ctx = createContext<AppCtx | null>(null);

export function useApp(): AppCtx {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useApp outside provider");
  return ctx;
}

let toastSeq = 1;

export function AppStateProvider({ children }: { children: React.ReactNode }) {
  const [view, setView] = useState<ViewId>("dashboard");
  const [theme, setTheme] = useState<"dark" | "light">("dark");
  const [volumes, setVolumes] = useState<VolumeInfo[]>([]);
  const [selectedVolume, setSelectedVolume] = useState("");
  const [scanning, setScanning] = useState(false);
  const [scanProgress, setScanProgress] = useState<ScanProgress | null>(null);
  const [lastScan, setLastScan] = useState<ScanSummary | null>(null);
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [opportunities, setOpportunities] = useState<CleanupOpportunity[]>([]);
  const [elevated, setElevated] = useState(false);
  const scanEpoch = useRef(0);

  const toast = useCallback((text: string, kind: Toast["kind"] = "info") => {
    const id = toastSeq++;
    setToasts((t) => [...t.slice(-4), { id, text, kind }]);
    window.setTimeout(() => {
      setToasts((t) => t.filter((x) => x.id !== id));
    }, 5200);
  }, []);

  const dismissToast = useCallback((id: number) => {
    setToasts((t) => t.filter((x) => x.id !== id));
  }, []);

  const navigate = useCallback((v: ViewId) => setView(v), []);

  const toggleTheme = useCallback(() => {
    setTheme((t) => {
      const next = t === "dark" ? "light" : "dark";
      document.documentElement.dataset.theme = next;
      return next;
    });
  }, []);

  const refreshVolumes = useCallback(async () => {
    try {
      const vols = await ipc.listVolumes();
      setVolumes(vols);
      setSelectedVolume((cur) => {
        if (cur && vols.some((v) => v.mountPoint === cur)) return cur;
        return vols.find((v) => v.isSystem)?.mountPoint ?? vols[0]?.mountPoint ?? "";
      });
    } catch (e) {
      toast(`Failed to list volumes: ${e}`, "error");
    }
  }, [toast]);

  const refreshElevation = useCallback(async () => {
    try {
      const s = await ipc.elevationStatus();
      setElevated(s.elevated);
    } catch {
      setElevated(false);
    }
  }, []);

  const refreshScanState = useCallback(async () => {
    try {
      const [latest, status] = await Promise.all([ipc.latestScan(), ipc.scanStatus()]);
      setLastScan(latest);
      setScanning(status.active);
    } catch {
      /* backend may not be present during frontend dev */
    }
  }, []);

  const invalidateScan = useCallback(() => {
    scanEpoch.current += 1;
    setOpportunities([]);
    void refreshScanState();
    void refreshVolumes();
  }, [refreshScanState, refreshVolumes]);

  // ---- initial load + event subscriptions --------------------------------
  useEffect(() => {
    document.documentElement.dataset.theme = "dark";
    void refreshVolumes();
    void refreshScanState();
    void refreshElevation();
    ipc.getSettings().then((s) => {
      setSettings(s);
      if (s.theme === "light") {
        setTheme("light");
        document.documentElement.dataset.theme = "light";
      }
    }).catch(() => undefined);

    const unlisteners: Array<() => void> = [];
    void listen<ScanProgress>("scan-progress", (p) => {
      setScanning(!["done", "failed", "cancelled"].includes(p.phase));
      setScanProgress(p);
    }).then((u) => unlisteners.push(u));
    void listen<ScanSummary>("scan-complete", (s) => {
      setScanning(false);
      setScanProgress(null);
      setLastScan(s);
      setOpportunities([]);
      toast(`Scan of ${s.volume} finished in ${Math.round(s.durationMs / 1000)}s`, "ok");
    }).then((u) => unlisteners.push(u));
    void listen<string>("scan-failed", (msg) => {
      setScanning(false);
      setScanProgress(null);
      toast(`Scan failed: ${msg}`, "error");
    }).then((u) => unlisteners.push(u));

    return () => unlisteners.forEach((u) => u());
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const startScan = useCallback(async (volume: string) => {
    const res = await ipc.startScan(volume);
    if (!res.accepted) {
      toast(res.reason ?? "Scan could not start", "error");
      return;
    }
    setScanning(true);
    setView("treemap");
  }, [toast]);

  const cancelScan = useCallback(() => {
    void ipc.cancelScan();
  }, []);

  const saveSettings = useCallback(async (s: AppSettings) => {
    setSettings(s);
    await ipc.saveSettings(s);
  }, []);

  const value = useMemo<AppCtx>(
    () => ({
      view, navigate, theme, toggleTheme,
      volumes, refreshVolumes,
      selectedVolume, setSelectedVolume,
      scanning, scanProgress, lastScan,
      startScan, cancelScan, invalidateScan,
      settings, saveSettings,
      toasts, toast, dismissToast,
      opportunities, setOpportunities,
      elevated, refreshElevation,
    }),
    [view, navigate, theme, toggleTheme, volumes, refreshVolumes, selectedVolume,
      scanning, scanProgress, lastScan, startScan, cancelScan, invalidateScan,
      settings, saveSettings, toasts, toast, dismissToast, opportunities,
      elevated, refreshElevation],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
