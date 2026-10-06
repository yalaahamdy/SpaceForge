/** Old Files, History, Insights, Why-Full, Activity views. */

import React, { useEffect, useState } from "react";
import { ipc } from "../api/ipc";
import type {
  CleanupHistoryEntry, FileEntry, HistoryPoint, Recommendation,
  SnapshotDiff, SnapshotMeta, StorageScore,
} from "../api/types";
import { formatBytes, formatCount, formatDateTime, formatDuration } from "../lib/format";
import { useApp } from "../state/AppState";
import {
  Column, ContextMenu, EmptyState, Sparkline, usePathActions, VirtualTable,
} from "../components/common/UI";
import { IconHistory, IconInsights, IconQuestion } from "../components/common/icons";

// ---------------------------------------------------------------------------
// Old files (spec §18)

export function OldFilesView() {
  const { lastScan, scanning } = useApp();
  const [days, setDays] = useState(180);
  const [rows, setRows] = useState<FileEntry[]>([]);
  const [menu, setMenu] = useState<{ x: number; y: number; path: string } | null>(null);
  const pathActions = usePathActions();

  useEffect(() => {
    if (!lastScan) return;
    ipc.oldFiles(days, 400).then(setRows).catch(() => setRows([]));
  }, [lastScan, days]);

  if (scanning) return <EmptyState title="Scanning…" />;
  if (!lastScan) return <EmptyState title="No scan available" />;

  const total = rows.reduce((a, b) => a + b.size, 0);

  const cols: Column<FileEntry>[] = [
    {
      key: "name", label: "File", width: "minmax(240px,1fr)", sortValue: (f) => f.name.toLowerCase(),
      render: (f) => <span title={f.path}>{f.name}</span>,
    },
    {
      key: "path", label: "Location", width: "minmax(200px,0.9fr)", sortValue: (f) => f.path,
      render: (f) => <span className="dim mono">{f.path}</span>,
    },
    { key: "size", label: "Size", width: "100px", align: "right", sortValue: (f) => f.size,
      render: (f) => <span className="num">{formatBytes(f.size)}</span> },
    { key: "age", label: "Last modified", width: "120px", sortValue: (f) => f.modified ?? "",
      render: (f) => <span className="dim">{formatDateTime(f.modified)}</span> },
  ];

  return (
    <div style={{ height: "calc(100vh - 120px)", display: "flex", flexDirection: "column" }}>
      <h1 className="page-title">Old Files</h1>
      <p className="page-sub">
        Potentially stale files — old does not mean disposable. {rows.length > 0 && (
          <>{formatCount(rows.length)} files · <b className="num">{formatBytes(total)}</b></>
        )}
      </p>
      <div className="seg" style={{ marginBottom: 10 }} role="group" aria-label="Age threshold">
        {[30, 90, 180, 365, 730].map((d) => (
          <button key={d} className={days === d ? "active" : ""} onClick={() => setDays(d)}>
            {d < 365 ? `${d} days` : `${d / 365} year${d > 365 ? "s" : ""}`}
          </button>
        ))}
      </div>
      <div style={{ flex: 1, minHeight: 0 }}>
        <VirtualTable rows={rows} columns={cols} defaultSort="age"
          onRowContext={(row, x, y) => setMenu({ x, y, path: row.path })} />
      </div>
      {menu && (
        <ContextMenu x={menu.x} y={menu.y} onClose={() => setMenu(null)} items={[
          { label: "Open containing folder", action: () => void pathActions(menu.path, "folder") },
          { label: "Copy path", action: () => void pathActions(menu.path, "copy") },
        ]} />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Storage history (spec §45) + snapshots (spec §44)

export function HistoryView() {
  const { lastScan, selectedVolume, volumes, toast } = useApp();
  const volume = lastScan?.volume ?? selectedVolume ?? volumes[0]?.mountPoint ?? "";
  const [points, setPoints] = useState<HistoryPoint[]>([]);
  const [snapshots, setSnapshots] = useState<SnapshotMeta[]>([]);
  const [diff, setDiff] = useState<SnapshotDiff | null>(null);
  const [picked, setPicked] = useState<number[]>([]);

  useEffect(() => {
    if (!volume) return;
    ipc.storageHistory(volume).then(setPoints).catch(() => setPoints([]));
    ipc.listSnapshots(volume).then(setSnapshots).catch(() => setSnapshots([]));
  }, [volume, lastScan]);

  function togglePick(id: number) {
    setPicked((p) => {
      const n = p.includes(id) ? p.filter((x) => x !== id) : [...p, id].slice(-2);
      if (n.length === 2) {
        void ipc.compareSnapshots(n[0], n[1]).then(setDiff).catch((e) => toast(String(e), "error"));
      } else {
        setDiff(null);
      }
      return n;
    });
  }

  async function saveSnapshot() {
    try {
      const name = `Snapshot ${new Date().toLocaleDateString()}`;
      await ipc.saveSnapshot(name);
      setSnapshots(await ipc.listSnapshots(volume));
      toast("Snapshot saved — pick two snapshots to compare.", "ok");
    } catch (e) {
      toast(String(e), "error");
    }
  }

  if (!lastScan) {
    return <EmptyState icon={<IconHistory size={38} />} title="No scan available"
      hint="History is built from real scans over time. Run scans and save snapshots to unlock growth analysis." />;
  }

  return (
    <div>
      <h1 className="page-title">Storage History</h1>
      <p className="page-sub">
        Timeline of {volume} built exclusively from recorded scans — no history is invented.
      </p>

      <div className="card">
        <h3>Used space over time</h3>
        {points.length >= 2 ? (
          <Sparkline points={points} width={640} height={150} />
        ) : (
          <p className="dim" style={{ fontSize: 13 }}>
            At least two scans are needed to draw a trend. Scan regularly (weekly is a good cadence)
            and this chart will answer “is my disk growing?”.
          </p>
        )}
      </div>

      <div className="card" style={{ marginTop: 14 }}>
        <div className="row">
          <h3 style={{ margin: 0 }}>Snapshots</h3>
          <div className="spacer" />
          <button className="btn small" onClick={() => void saveSnapshot()}>Save snapshot of current scan</button>
        </div>
        {snapshots.length === 0 ? (
          <p className="dim" style={{ fontSize: 13, marginTop: 8 }}>
            No snapshots yet. Snapshots freeze a scan for later comparison: “Downloads grew by
            17 GB since last month”.
          </p>
        ) : (
          <div style={{ marginTop: 8 }}>
            {snapshots.map((s) => (
              <div key={s.id} className="row" style={{ padding: "6px 0", borderBottom: "1px solid var(--stroke-1)" }}>
                <input type="checkbox" checked={picked.includes(s.id)} onChange={() => togglePick(s.id)}
                  aria-label={`Select ${s.name} for comparison`} />
                <b style={{ fontSize: 13 }}>{s.name}</b>
                <span className="dim" style={{ fontSize: 12 }}>{formatDateTime(s.createdAt)}</span>
                <div className="spacer" />
                <span className="num dim">{formatBytes(s.totalSize)} · {formatCount(s.totalFiles)} files</span>
              </div>
            ))}
            <p className="dim3" style={{ fontSize: 12 }}>Select exactly two snapshots to compare.</p>
          </div>
        )}
      </div>

      {diff && (
        <div className="grid-2" style={{ marginTop: 14 }}>
          <div className="card">
            <h3>Growth: {formatBytes(Math.abs(diff.totalDelta))} {diff.totalDelta >= 0 ? "added" : "removed"}</h3>
            {diff.grew.slice(0, 12).map((d) => (
              <div key={d.path} className="row" style={{ fontSize: 12.5, padding: "3px 0" }}>
                <span className="mono dim" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{d.path}</span>
                <div className="spacer" />
                <span className="num" style={{ color: "var(--warn)" }}>+{formatBytes(d.delta)}</span>
              </div>
            ))}
            {diff.grew.length === 0 && <p className="dim" style={{ fontSize: 13 }}>No growing folders recorded.</p>}
          </div>
          <div className="card">
            <h3>New large files</h3>
            {diff.newLargeFiles.slice(0, 12).map((f) => (
              <div key={f.path} className="row" style={{ fontSize: 12.5, padding: "3px 0" }}>
                <span className="mono dim" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{f.path}</span>
                <div className="spacer" />
                <span className="num">{formatBytes(f.size)}</span>
              </div>
            ))}
            {diff.newLargeFiles.length === 0 && <p className="dim" style={{ fontSize: 13 }}>No new large files.</p>}
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Space Insights (spec §34, §46–47, §58)

export function InsightsView() {
  const { lastScan, opportunities } = useApp();
  const [score, setScore] = useState<StorageScore | null>(null);
  const [recs, setRecs] = useState<Recommendation[]>([]);

  useEffect(() => {
    if (!lastScan) return;
    ipc.storageScore().then(setScore).catch(() => undefined);
    ipc.recommendations(opportunities).then(setRecs).catch(() => undefined);
  }, [lastScan, opportunities]);

  if (!lastScan) return <EmptyState icon={<IconInsights size={38} />} title="No scan available" />;

  return (
    <div>
      <h1 className="page-title">Space Insights</h1>
      <p className="page-sub">Evidence-based diagnostics — every statement is backed by scan or measurement data.</p>

      <div className="grid-3">
        <div className="kpi accent">
          <div className="label">Storage pressure</div>
          <div className="value">{score?.pressure ?? "—"}</div>
        </div>
        <div className="kpi amber">
          <div className="label">Reclaimable</div>
          <div className="value">{formatBytes(score?.reclaimableBytes ?? 0)}</div>
        </div>
        <div className="kpi">
          <div className="label">Duplicate waste</div>
          <div className="value">{formatBytes(score?.duplicateWaste ?? 0)}</div>
        </div>
      </div>

      <div className="card" style={{ marginTop: 14 }}>
        <h3>Why this rating</h3>
        {(score?.explanations ?? ["Run a scan to compute the storage score."]).map((e, i) => (
          <p key={i} className="dim" style={{ fontSize: 13.5, margin: "6px 0" }}>{e}</p>
        ))}
      </div>

      <div className="card" style={{ marginTop: 14 }}>
        <h3>Recommended actions</h3>
        {recs.length === 0 && <p className="dim" style={{ fontSize: 13 }}>Nothing flagged right now.</p>}
        {recs.map((r, i) => (
          <div key={i} className="row" style={{ padding: "6px 0", borderBottom: "1px solid var(--stroke-1)" }}>
            <span className={`badge ${r.priority === "high" ? "safe" : r.priority === "medium" ? "low" : "review"}`}>
              {r.priority}
            </span>
            <div>
              <div style={{ fontWeight: 600, fontSize: 13.5 }}>{r.title}</div>
              <div className="dim" style={{ fontSize: 12.5 }}>{r.detail}</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Why is my disk full? (spec §61)

export function WhyFullView() {
  const { lastScan } = useApp();
  const [score, setScore] = useState<StorageScore | null>(null);
  const [top, setTop] = useState<[string, number, number][]>([]);

  useEffect(() => {
    if (!lastScan) return;
    ipc.storageScore().then(setScore).catch(() => undefined);
    ipc.topConsumers().then((t) => setTop(t.topFolders.slice(0, 5))).catch(() => undefined);
  }, [lastScan]);

  if (!lastScan) {
    return <EmptyState icon={<IconQuestion size={40} />} title="Scan first"
      hint="This diagnosis is generated from real scan data — no speculation, no invented statistics." />;
  }

  const usedPct = lastScan.volumeTotal > 0 ? (lastScan.volumeUsed / lastScan.volumeTotal) * 100 : 0;
  const topThree = top.slice(0, 3);

  return (
    <div>
      <h1 className="page-title">Why is my disk full?</h1>
      <p className="page-sub">A factual diagnosis from the latest scan of {lastScan.volume}.</p>

      <div className="card" style={{ padding: 22 }}>
        <p style={{ fontSize: 16, lineHeight: 1.7, margin: 0 }}>
          Your {lastScan.volume} drive is <b className="num">{usedPct.toFixed(1)}% full</b> —{" "}
          {formatBytes(lastScan.volumeUsed)} used, {formatBytes(lastScan.volumeFree)} free.
        </p>
        {topThree.length > 0 && (
          <>
            <p style={{ fontSize: 14.5, marginTop: 14, marginBottom: 6 }}>
              The largest contributors are:
            </p>
            {topThree.map(([path, size], i) => (
              <p key={path} style={{ margin: "4px 0 4px 14px", fontSize: 14 }}>
                <b className="num" style={{ color: "var(--accent)" }}>{i + 1}.</b>{" "}
                <span className="mono" style={{ fontSize: 13 }}>{path}</span> —{" "}
                <b className="num">{formatBytes(size)}</b>
              </p>
            ))}
          </>
        )}
        {score && score.reclaimableBytes > 0 && (
          <>
            <p style={{ fontSize: 14.5, marginTop: 16, marginBottom: 4 }}>
              Potentially reclaimable: <b className="num" style={{ color: "var(--accent-2)" }}>
                {formatBytes(score.reclaimableBytes)}
              </b>
            </p>
            <p className="dim" style={{ fontSize: 13, margin: 0 }}>
              Measured from safe/low-risk cleanup discovery — browser caches, temporary data,
              Recycle Bin and similar. Review the Cleanup Center for the breakdown.
            </p>
          </>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Cleanup / activity history (spec §50)

export function ActivityView() {
  const { toast } = useApp();
  const [rows, setRows] = useState<CleanupHistoryEntry[]>([]);

  useEffect(() => {
    ipc.cleanupHistory().then(setRows).catch(() => setRows([]));
  }, []);

  return (
    <div>
      <h1 className="page-title">Cleanup History</h1>
      <p className="page-sub">Every cleanup action is recorded locally — what ran, when, and what it reclaimed.</p>

      {rows.length === 0 ? (
        <EmptyState icon={<IconHistory size={38} />} title="No cleanups recorded yet"
          hint="After your first cleanup, this page shows estimated vs actual reclaim, failures and skipped items." />
      ) : (
        <div className="card">
          {rows.map((r) => (
            <div key={r.id} className="row" style={{ padding: "9px 0", borderBottom: "1px solid var(--stroke-1)" }}>
              <div style={{ minWidth: 0 }}>
                <div style={{ fontWeight: 600, fontSize: 13.5 }}>{formatDateTime(r.at)} — {formatBytes(r.actualReclaim)} reclaimed</div>
                <div className="dim" style={{ fontSize: 12 }}>
                  {formatCount(r.itemCount)} items · {r.durationMs}ms
                  {r.failedItems > 0 && ` · ${r.failedItems} failed`}
                  {r.skippedItems > 0 && ` · ${r.skippedItems} skipped`}
                </div>
              </div>
              <div className="spacer" />
              <span className="badge low">{r.categories.length} categories</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
