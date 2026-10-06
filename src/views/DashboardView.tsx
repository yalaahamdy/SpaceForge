/** Dashboard — instant storage diagnosis (spec §7, §60). */

import React, { useEffect, useState } from "react";
import { ipc } from "../api/ipc";
import type { CleanupOpportunity, Recommendation, StorageScore } from "../api/types";
import { useApp } from "../state/AppState";
import { formatBytes, formatCount, formatDateTime, formatETA, formatTimer } from "../lib/format";
import { EmptyState, Modal, RiskBadge } from "../components/common/UI";
import {
  IconCamera, IconChevronRight, IconDrive, IconPlay, IconShield, IconStop,
} from "../components/common/icons";

/** Animated Circular Ring Gauge for disk capacity */
function CircularGauge({
  percent,
  size = 60,
  strokeWidth = 5.5,
}: {
  percent: number;
  size?: number;
  strokeWidth?: number;
}) {
  const radius = (size - strokeWidth) / 2;
  const circumference = radius * 2 * Math.PI;
  const clamped = Math.min(100, Math.max(0, percent));
  const offset = circumference - (clamped / 100) * circumference;
  const isHigh = percent > 90;
  const isMed = percent > 75;
  const strokeColor = isHigh ? "var(--danger)" : isMed ? "var(--accent-2)" : "var(--accent)";

  return (
    <div
      style={{
        position: "relative",
        width: size,
        height: size,
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        flexShrink: 0,
      }}
    >
      <svg width={size} height={size} style={{ transform: "rotate(-90deg)" }}>
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke="var(--bg-s3)"
          strokeWidth={strokeWidth}
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke={strokeColor}
          strokeWidth={strokeWidth}
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          strokeLinecap="round"
          style={{ transition: "stroke-dashoffset 0.8s cubic-bezier(0.16, 1, 0.3, 1)" }}
        />
      </svg>
      <div style={{ position: "absolute", textAlign: "center", pointerEvents: "none" }}>
        <span className="num" style={{ fontSize: 12, fontWeight: 700, color: "var(--text-1)" }}>
          {clamped.toFixed(0)}%
        </span>
      </div>
    </div>
  );
}

/** High-Performance Live Scanning Monitor with Real-Time Throughput & Timers */
function LiveScanMonitor({
  progress,
  activeVolume,
  onCancel,
}: {
  progress: import("../api/types").ScanProgress | null;
  activeVolume?: import("../api/types").VolumeInfo;
  onCancel: () => void;
}) {
  const elapsedSec = (progress?.elapsedMs ?? 0) / 1000;
  const bytesSeen = progress?.bytesSeen ?? 0;
  const filesSeen = progress?.filesSeen ?? 0;
  const dirsSeen = progress?.dirsSeen ?? 0;

  // Real-time throughput calculations
  const bytesPerSec = elapsedSec > 0.4 ? bytesSeen / elapsedSec : 0;
  const filesPerSec = elapsedSec > 0.4 ? filesSeen / elapsedSec : 0;

  // Progress computation
  const targetBytes = activeVolume?.usedBytes || 0;
  const pct = targetBytes > 0 ? Math.min(99, Math.max(1, (bytesSeen / targetBytes) * 100)) : 45;

  // ETA computation
  const remainingBytes = Math.max(0, targetBytes - bytesSeen);
  const etaSec = bytesPerSec > 1024 * 512 ? remainingBytes / bytesPerSec : 0;

  return (
    <div
      className="card"
      style={{
        marginTop: 16,
        borderColor: "var(--accent)",
        position: "relative",
        overflow: "hidden",
        boxShadow: "0 0 24px rgba(79, 142, 247, 0.12)",
      }}
    >
      {/* Top Header */}
      <div className="row" style={{ alignItems: "center", gap: 10 }}>
        <span className="scan-pulse" />
        <h3 style={{ margin: 0, textTransform: "none", fontSize: 15, color: "var(--text-1)", fontWeight: 700 }}>
          Scanning Drive {progress?.volume || ""} · High-Performance Direct I/O
        </h3>
        <div className="spacer" />
        <span className="cosmic-chip" style={{ color: "var(--ok)", borderColor: "rgba(16, 185, 129, 0.4)" }}>
          <span className="chip-dot" style={{ background: "var(--ok)" }} />
          Rayon Parallel Engine Active
        </span>
        <button className="btn small danger" onClick={onCancel}>
          <IconStop size={12} /> Cancel
        </button>
      </div>

      {/* Progress Bar with glowing tip & percentage */}
      <div style={{ margin: "16px 0 6px" }}>
        <div className="row" style={{ fontSize: 12, marginBottom: 6 }}>
          <span style={{ fontWeight: 600, color: "var(--accent)" }}>
            Phase: {progress?.phase === "walking" ? "Parallel Tree Walk & Indexing" : progress?.phase || "Walking"}
          </span>
          <div className="spacer" />
          <span className="num" style={{ fontWeight: 700, fontSize: 14, color: "var(--text-1)" }}>
            {pct.toFixed(1)}% Complete
          </span>
        </div>

        <div className="bar-track" style={{ height: 10, borderRadius: 99, background: "var(--bg-s3)" }}>
          <div
            className="bar-fill"
            style={{
              width: `${pct}%`,
              background: "linear-gradient(90deg, var(--accent) 0%, #60a5fa 70%, var(--accent-2) 100%)",
              boxShadow: "0 0 12px rgba(79, 142, 247, 0.6)",
            }}
          />
        </div>
      </div>

      {/* 4 Professional Live Telemetry Counters */}
      <div className="grid-4" style={{ marginTop: 14, gap: 10 }}>
        <div
          style={{
            background: "var(--bg-s2)",
            padding: "10px 14px",
            borderRadius: "var(--radius-s)",
            border: "1px solid var(--stroke-1)",
          }}
        >
          <div className="dim3" style={{ fontSize: 11, fontWeight: 700, textTransform: "uppercase" }}>
            Throughput Speed
          </div>
          <div className="num" style={{ fontSize: 18, fontWeight: 700, color: "var(--accent)", marginTop: 2 }}>
            {formatBytes(bytesPerSec)}/s
          </div>
          <div className="dim num" style={{ fontSize: 11 }}>
            {Math.round(filesPerSec).toLocaleString()} files/sec
          </div>
        </div>

        <div
          style={{
            background: "var(--bg-s2)",
            padding: "10px 14px",
            borderRadius: "var(--radius-s)",
            border: "1px solid var(--stroke-1)",
          }}
        >
          <div className="dim3" style={{ fontSize: 11, fontWeight: 700, textTransform: "uppercase" }}>
            Data Indexed
          </div>
          <div className="num" style={{ fontSize: 18, fontWeight: 700, marginTop: 2 }}>
            {formatBytes(bytesSeen)}
          </div>
          <div className="dim num" style={{ fontSize: 11 }}>
            {formatCount(filesSeen)} files · {formatCount(dirsSeen)} folders
          </div>
        </div>

        <div
          style={{
            background: "var(--bg-s2)",
            padding: "10px 14px",
            borderRadius: "var(--radius-s)",
            border: "1px solid var(--stroke-1)",
          }}
        >
          <div className="dim3" style={{ fontSize: 11, fontWeight: 700, textTransform: "uppercase" }}>
            Elapsed Time
          </div>
          <div className="num" style={{ fontSize: 18, fontWeight: 700, color: "var(--text-1)", marginTop: 2 }}>
            {formatTimer(progress?.elapsedMs ?? 0)}
          </div>
          <div className="dim" style={{ fontSize: 11 }}>live elapsed timer</div>
        </div>

        <div
          style={{
            background: "var(--bg-s2)",
            padding: "10px 14px",
            borderRadius: "var(--radius-s)",
            border: "1px solid var(--stroke-1)",
          }}
        >
          <div className="dim3" style={{ fontSize: 11, fontWeight: 700, textTransform: "uppercase" }}>
            Estimated Time (ETA)
          </div>
          <div className="num" style={{ fontSize: 18, fontWeight: 700, color: "var(--accent-2)", marginTop: 2 }}>
            {formatETA(etaSec)}
          </div>
          <div className="dim num" style={{ fontSize: 11 }}>adaptive bandwidth model</div>
        </div>
      </div>

      {/* Real-time file path terminal ticker */}
      <div
        style={{
          marginTop: 12,
          background: "var(--bg-s3)",
          padding: "7px 12px",
          borderRadius: "var(--radius-s)",
          display: "flex",
          alignItems: "center",
          gap: 8,
          border: "1px solid var(--stroke-1)",
        }}
      >
        <span className="dim3" style={{ fontSize: 10.5, fontWeight: 700, flexShrink: 0 }}>
          TRAVERSING:
        </span>
        <span
          className="mono dim"
          style={{ fontSize: 11.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
        >
          {progress?.currentPath || "indexing directory trees…"}
        </span>
      </div>
    </div>
  );
}

export function DashboardView() {
  const {
    volumes, selectedVolume, setSelectedVolume, scanning, scanProgress,
    lastScan, startScan, cancelScan, opportunities, setOpportunities,
    navigate, toast, elevated,
  } = useApp();
  const [score, setScore] = useState<StorageScore | null>(null);
  const [recs, setRecs] = useState<Recommendation[]>([]);
  const [discovering, setDiscovering] = useState(false);
  const [detail, setDetail] = useState<null | { title: string; lines: string[] }>(null);

  const active = volumes.find((v) => v.mountPoint === selectedVolume);

  useEffect(() => {
    if (!lastScan) return;
    ipc.storageScore().then(setScore).catch(() => undefined);
  }, [lastScan, opportunities]);

  useEffect(() => {
    if (opportunities.length === 0) return;
    ipc.recommendations(opportunities).then(setRecs).catch(() => undefined);
  }, [opportunities]);

  async function discover() {
    setDiscovering(true);
    try {
      const opps = await ipc.discoverCleanups();
      setOpportunities(opps);
      if (opps.length === 0) toast("No cleanup opportunities found — storage looks good.", "ok");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setDiscovering(false);
    }
  }

  const safeTotal = opportunities
    .filter((o) => (o.rule.risk === "safe" || o.rule.risk === "low") && !o.blockedReason)
    .reduce((a, b) => a + b.reclaimableSize, 0);
  const largeTotal = opportunities
    .filter((o) => o.rule.id.startsWith("win.") === false)
    .reduce((a, b) => a + b.currentSize, 0);

  // Breakdown segments calculation for active drive
  const totalBytes = active?.totalBytes || 1;
  const usedBytes = active?.usedBytes || 0;
  const freeBytes = active?.freeBytes || 0;
  const safeBytes = Math.min(safeTotal, usedBytes);
  const netUsedBytes = Math.max(0, usedBytes - safeBytes);

  const usedPct = ((netUsedBytes / totalBytes) * 100).toFixed(1);
  const safePct = ((safeBytes / totalBytes) * 100).toFixed(1);
  const freePct = ((freeBytes / totalBytes) * 100).toFixed(1);

  return (
    <div className="view-container">
      {/* Header Toolbar */}
      <div className="row-wrap" style={{ marginBottom: 6, gap: 12 }}>
        <div>
          <h1 className="page-title" style={{ margin: 0 }}>Dashboard</h1>
          <div className="row" style={{ gap: 8, marginTop: 4 }}>
            <span className="cosmic-chip">
              <span
                className="chip-dot"
                style={{ background: elevated ? "var(--ok)" : "var(--warn)" }}
              />
              {elevated ? "Administrator — Full Deep Access" : "Standard User Mode"}
            </span>
            {active && (
              <span className="cosmic-chip">
                <span className="chip-dot" />
                {active.mountPoint} ({active.filesystem})
              </span>
            )}
          </div>
        </div>

        <div className="spacer" />

        <div className="row" style={{ gap: 10 }}>
          {scanning ? (
            <button className="btn danger" onClick={cancelScan}>
              <IconStop size={14} /> Cancel scan
            </button>
          ) : (
            <>
              <button className="btn" onClick={() => void discover()} disabled={discovering}>
                {discovering ? "Discovering…" : "Find reclaimable space"}
              </button>
              <button
                className="btn primary glow"
                onClick={() => void startScan(selectedVolume)}
              >
                <IconPlay size={13} /> Scan {selectedVolume || ""}
              </button>
            </>
          )}
        </div>
      </div>

      <p className="page-sub" style={{ marginBottom: 16 }}>
        {active
          ? `${active.mountPoint} — ${active.label || "Local Drive"} · ${active.filesystem}${active.isSystem ? " · System Disk" : ""}`
          : "No volume selected"}
      </p>

      {/* Drive cards grid */}
      <div
        className="grid-4"
        style={{ gridTemplateColumns: `repeat(${Math.min(volumes.length, 4)}, 1fr)` }}
      >
        {volumes.slice(0, 8).map((v) => {
          const selected = v.mountPoint === selectedVolume;
          const pct = Math.min(100, v.usedPercent);
          return (
            <button
              key={v.mountPoint}
              className="kpi"
              onClick={() => setSelectedVolume(v.mountPoint)}
              style={{
                cursor: "pointer",
                textAlign: "left",
                width: "100%",
                borderColor: selected ? "var(--accent)" : undefined,
                boxShadow: selected ? "0 0 0 1px var(--accent), var(--shadow-accent)" : undefined,
              }}
            >
              <div className="row" style={{ gap: 6 }}>
                <span className="label">
                  {v.mountPoint} {v.isSystem ? "· System" : v.kind === "removable" ? "· Removable" : ""}
                </span>
                <div className="spacer" />
                <span
                  className="cosmic-chip"
                  style={{ fontSize: 10, padding: "1px 6px" }}
                >
                  {v.media === "ssd" ? "SSD" : v.media === "hdd" ? "HDD" : "DISK"}
                </span>
              </div>

              <div className="row" style={{ marginTop: 8, alignItems: "center", gap: 12 }}>
                <CircularGauge percent={pct} size={54} strokeWidth={5} />
                <div style={{ minWidth: 0, flex: 1 }}>
                  <div className="value" style={{ fontSize: 18, marginTop: 0 }}>
                    {formatBytes(v.totalBytes)}
                  </div>
                  <div className="sub num" style={{ fontSize: 11 }}>
                    {formatBytes(v.freeBytes)} free
                  </div>
                </div>
              </div>

              <div className="bar-track" style={{ marginTop: 10 }}>
                <div
                  className={`bar-fill${pct > 90 ? " danger" : pct > 75 ? " amber" : ""}`}
                  style={{ width: `${pct}%` }}
                />
              </div>

              <div className="row" style={{ fontSize: 11, marginTop: 6, color: "var(--text-3)" }}>
                <span className="num">{formatBytes(v.usedBytes)} used</span>
                <div className="spacer" />
                <span className="num">{v.usedPercent.toFixed(1)}% full</span>
              </div>
            </button>
          );
        })}
      </div>

      {/* Hero Action & Storage Diagnosis Banner */}
      {lastScan ? (
        <div className="hero-banner">
          <div className="hero-glow" />
          <div className="row" style={{ alignItems: "flex-start" }}>
            <div style={{ flex: 1 }}>
              <div className="row" style={{ gap: 8 }}>
                <span className="badge safe">DIAGNOSIS COMPLETE</span>
                <span className="dim3 num" style={{ fontSize: 12 }}>
                  Scanned in {Math.round(lastScan.durationMs / 1000)}s
                </span>
              </div>
              <h2 style={{ fontSize: 18, margin: "8px 0 4px", color: "var(--text-1)" }}>
                Drive {lastScan.volume} is{" "}
                <span style={{ color: lastScan.volumeUsed / (lastScan.volumeTotal || 1) > 0.85 ? "var(--warn)" : "var(--accent)" }}>
                  {lastScan.volumeTotal > 0
                    ? ((lastScan.volumeUsed / lastScan.volumeTotal) * 100).toFixed(1)
                    : "?"}% full
                </span>
              </h2>
              <p className="dim" style={{ margin: "0 0 10px", fontSize: 13 }}>
                Walked <b className="num text-1">{formatCount(lastScan.totalFiles)} files</b> across{" "}
                <b className="num text-1">{formatCount(lastScan.totalDirs)} folders</b>{" "}
                ({formatBytes(lastScan.durationMs >= 0 ? lastScan.totalSize : 0)} analyzed).
              </p>

              {/* Multi-Breakdown Bar */}
              <div className="multi-breakdown-bar">
                <div
                  className="seg-slice"
                  style={{ width: `${usedPct}%`, background: "var(--accent)" }}
                  title={`Allocated Data: ${formatBytes(netUsedBytes)} (${usedPct}%)`}
                />
                {safeTotal > 0 && (
                  <div
                    className="seg-slice"
                    style={{ width: `${safePct}%`, background: "var(--accent-2)" }}
                    title={`Safe Reclaimable: ${formatBytes(safeBytes)} (${safePct}%)`}
                  />
                )}
                <div
                  className="seg-slice"
                  style={{ width: `${freePct}%`, background: "var(--stroke-2)" }}
                  title={`Free Space: ${formatBytes(freeBytes)} (${freePct}%)`}
                />
              </div>

              {/* Breakdown Legend */}
              <div className="row-wrap" style={{ gap: 14, fontSize: 11.5, marginTop: 8 }}>
                <span className="row" style={{ gap: 5 }}>
                  <span className="chip-dot" style={{ background: "var(--accent)" }} />
                  Allocated Data ({formatBytes(netUsedBytes)})
                </span>
                {safeTotal > 0 && (
                  <span className="row" style={{ gap: 5, color: "var(--accent-2)" }}>
                    <span className="chip-dot" style={{ background: "var(--accent-2)" }} />
                    Safe Reclaimable ({formatBytes(safeTotal)})
                  </span>
                )}
                <span className="row" style={{ gap: 5, color: "var(--text-3)" }}>
                  <span className="chip-dot" style={{ background: "var(--stroke-2)" }} />
                  Free Space ({formatBytes(freeBytes)})
                </span>
              </div>
            </div>
          </div>

          {/* Quick Action Navigation Buttons */}
          <div className="row" style={{ gap: 10, marginTop: 16 }}>
            <button className="btn small" onClick={() => navigate("analyzer")}>
              Open Disk Analyzer <IconChevronRight size={13} />
            </button>
            <button className="btn small" onClick={() => navigate("treemap")}>
              Open Storage Map <IconChevronRight size={13} />
            </button>
            {safeTotal > 0 && (
              <button
                className="btn small amber"
                onClick={() => navigate("cleanup-center")}
                style={{ fontWeight: 650 }}
              >
                Review safe cleanup ({formatBytes(safeTotal)}) <IconChevronRight size={13} />
              </button>
            )}
          </div>
        </div>
      ) : scanning ? (
        <LiveScanMonitor
          progress={scanProgress}
          activeVolume={active}
          onCancel={cancelScan}
        />
      ) : (
        <div className="card" style={{ marginTop: 16 }}>
          <EmptyState
            icon={<IconDrive size={40} />}
            title="Start Storage Analysis"
            hint="Run a fast multi-threaded scan to discover what is taking up space, find safe cleanup opportunities, and inspect duplicate files."
            action={
              <button
                className="btn primary glow"
                onClick={() => void startScan(selectedVolume)}
              >
                <IconPlay size={13} /> Scan {selectedVolume || "Disk"}
              </button>
            }
          />
        </div>
      )}

      {/* KPI row */}
      <div className="grid-4" style={{ marginTop: 16 }}>
        <div className="kpi accent">
          <div className="label">Storage pressure</div>
          <div className="value" style={{ fontSize: 20 }}>{score?.pressure ?? "—"}</div>
          <div className="sub">{score ? `${score.freePercent.toFixed(1)}% free space` : "scan to evaluate"}</div>
        </div>
        <div className="kpi amber">
          <div className="label">High-confidence reclaimable</div>
          <div className="value" style={{ fontSize: 20 }}>{formatBytes(safeTotal)}</div>
          <div className="sub">{opportunities.length} opportunities detected</div>
        </div>
        <div className="kpi">
          <div className="label">Duplicate waste</div>
          <div className="value" style={{ fontSize: 20 }}>{formatBytes(score?.duplicateWaste ?? 0)}</div>
          <div className="sub">run Duplicates to detect</div>
        </div>
        <div className="kpi">
          <div className="label">Large-file content</div>
          <div className="value" style={{ fontSize: 20 }}>{formatBytes(largeTotal)}</div>
          <div className="sub">app & browser caches</div>
        </div>
      </div>

      {/* Recommendations */}
      {recs.length > 0 && (
        <div className="card" style={{ marginTop: 16 }}>
          <h3>Smart Recommendations</h3>
          <div className="stack" style={{ gap: 8 }}>
            {recs.slice(0, 6).map((r, i) => (
              <div
                key={i}
                className="row"
                style={{
                  alignItems: "flex-start",
                  padding: "8px 10px",
                  borderRadius: "var(--radius-s)",
                  background: "var(--bg-s2)",
                  border: "1px solid var(--stroke-1)",
                }}
              >
                <span
                  className={`badge ${r.priority === "high" ? "safe" : r.priority === "medium" ? "low" : "review"}`}
                  style={{ marginTop: 2 }}
                >
                  {r.priority === "high" ? "High Impact" : r.priority === "medium" ? "Medium" : "Review"}
                </span>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontWeight: 600, fontSize: 13.5 }}>{r.title}</div>
                  <div className="dim" style={{ fontSize: 12 }}>{r.detail}</div>
                </div>
                <div className="spacer" />
                <button className="btn small ghost" onClick={() => navigate(r.target as never)}>
                  Open →
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {detail && (
        <Modal
          title={detail.title}
          onClose={() => setDetail(null)}
          footer={<button className="btn" onClick={() => setDetail(null)}>Close</button>}
        >
          {detail.lines.map((l, i) => (
            <p key={i} className="dim" style={{ userSelect: "text" }}>{l}</p>
          ))}
        </Modal>
      )}

      {/* Volume detail table */}
      {active && (
        <div className="card" style={{ marginTop: 16 }}>
          <h3>Volume Technical Specifications</h3>
          <div className="grid-4" style={{ fontSize: 13 }}>
            <div>
              <span className="dim3">Filesystem</span>
              <div style={{ fontWeight: 600 }}>{active.filesystem}</div>
            </div>
            <div>
              <span className="dim3">Media Kind</span>
              <div style={{ fontWeight: 600 }}>{active.kind}{active.readOnly ? " · Read-only" : ""}</div>
            </div>
            <div>
              <span className="dim3">Scan Eligibility</span>
              <div style={{ fontWeight: 600 }}>{active.scanEligible ? "Fully Eligible" : active.ineligibleReason}</div>
            </div>
            <div>
              <span className="dim3">Last Diagnostic Scan</span>
              <div style={{ fontWeight: 600 }}>{formatDateTime(lastScan?.startedAt ?? null)}</div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
