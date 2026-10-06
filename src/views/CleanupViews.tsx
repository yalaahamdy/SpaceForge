/** Cleanup views (spec §20–§36, §62, §63): Cleanup Center + family views.
 *
 * One engine powers all cleanup pages; each page filters the discovered
 * opportunities by family and renders the same preview/execute flow.
 */

import React, { useCallback, useEffect, useState } from "react";
import { ipc, listen } from "../api/ipc";
import type {
  CleanupOpportunity, CleanupPlan, CleanupProfile, CleanupReport,
} from "../api/types";
import { formatBytes, formatCount, formatDateTime, formatDuration } from "../lib/format";
import { useApp } from "../state/AppState";
import { EmptyState, Modal, RiskBadge } from "../components/common/UI";
import { IconShield } from "../components/common/icons";

const PROFILE_INFO: Record<CleanupProfile, { label: string; blurb: string }> = {
  smart_clean: { label: "Smart Clean", blurb: "Only high-confidence safe cleanup." },
  standard_clean: { label: "Standard Clean", blurb: "Safe + selected low-risk cleanup." },
  deep_clean: { label: "Deep Clean", blurb: "Broader cleanup — everything is previewed, nothing silent." },
  custom: { label: "Custom", blurb: "You control every category individually." },
};

interface CleanupProgress { done: number; total: number; rule: string; planId: string }

export function CleanupView({ family }: { family: "all" | "windows" | "application" | "browser" | "developer" | "privacy" }) {
  const { opportunities, setOpportunities, toast, settings, lastScan } = useApp();
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [profile, setProfile] = useState<CleanupProfile>("smart_clean");
  const [plan, setPlan] = useState<CleanupPlan | null>(null);
  const [report, setReport] = useState<CleanupReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<CleanupProgress | null>(null);
  const [inspect, setInspect] = useState<CleanupOpportunity | null>(null);

  const titleByFamily: Record<string, string> = {
    all: "Cleanup Center",
    windows: "Windows Cleanup",
    application: "Application Cleanup",
    browser: "Browser Cleanup",
    developer: "Developer Cache Cleanup",
    privacy: "Privacy Cleanup",
  };

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const opps = await ipc.discoverCleanups();
      setOpportunities(opps);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setLoading(false);
    }
  }, [setOpportunities, toast]);

  useEffect(() => {
    if (opportunities.length === 0) void refresh();
    else setLoading(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const un = listen<CleanupProgress>("cleanup-progress", setProgress);
    return () => { void un.then((u) => u()); };
  }, []);

  const visible = opportunities.filter((o) =>
    family === "all" ? true : o.rule.family === family,
  );

  // Smart/standard profiles preselect safe sets; custom starts empty.
  useEffect(() => {
    const maxRisk = profile === "smart_clean" ? 1
      : profile === "standard_clean" ? 2
      : profile === "deep_clean" ? 3 : 0;
    const order: Record<string, number> = { safe: 1, low: 2, review: 3, advanced: 4, dangerous: 5 };
    const next = new Set<string>();
    if (profile !== "custom") {
      for (const o of visible) {
        if (order[o.rule.risk] <= maxRisk && !o.blockedReason && o.reclaimableSize > 0) {
          next.add(o.ruleId);
        }
      }
    }
    setSelected(next);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [profile, opportunities]);

  const selectedTotal = visible
    .filter((o) => selected.has(o.ruleId))
    .reduce((a, b) => a + b.reclaimableSize, 0);

  async function buildPlan() {
    setBusy(true);
    try {
      const p = await ipc.buildCleanupPlan(profile, [...selected]);
      setPlan(p);
      setReport(null);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setBusy(false);
    }
  }

  async function execute() {
    if (!plan) return;
    if (settings.confirmBeforeClean && profile !== "smart_clean") {
      const ok = window.confirm(
        `Clean ${formatBytes(plan.estimatedReclaim)} across ${plan.entries.length} categories?\n\n` +
        "Items marked Recyclable can be restored; permanent deletions cannot.",
      );
      if (!ok) return;
    }
    setBusy(true);
    try {
      const r = await ipc.executeCleanup(plan);
      setReport(r);
      setPlan(null);
      await refresh();
      toast(`Cleanup finished — ${formatBytes(r.actualReclaim)} reclaimed.`, "ok");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      setBusy(false);
      setProgress(null);
    }
  }

  const needsScanNotice = !lastScan && !loading;

  return (
    <div>
      <div className="row-wrap">
        <h1 className="page-title" style={{ margin: 0 }}>{titleByFamily[family]}</h1>
        <div className="spacer" />
        <button className="btn" onClick={() => void refresh()} disabled={loading}>
          {loading ? "Measuring…" : "Re-measure"}
        </button>
      </div>
      <p className="page-sub">
        Analyze first, delete second: every category shows what it is, why it is removable,
        its risk level and whether the action is reversible before anything is touched.
      </p>

      {needsScanNotice && (
        <div className="warning-box" style={{ marginBottom: 12 }}>
          Cleanup discovery works without a scan, but pairing it with a fresh scan gives better
          context for large-file and duplicate decisions.
        </div>
      )}

      {visible.length === 0 && !loading ? (
        <EmptyState
          icon={<IconShield size={38} />}
          title="No cleanup opportunities found"
          hint="Your storage is looking good. Re-measure later, or check Settings to enable opt-in developer and privacy cleaners."
        />
      ) : (
        <div className="stack">
          {/* Profile selector */}
          <div className="card">
            <h3>Cleanup profile</h3>
            <div className="seg" role="group" aria-label="Cleanup profile">
              {(Object.keys(PROFILE_INFO) as CleanupProfile[]).map((p) => (
                <button key={p} className={profile === p ? "active" : ""} onClick={() => setProfile(p)}>
                  {PROFILE_INFO[p].label}
                </button>
              ))}
            </div>
            <p className="dim" style={{ fontSize: 12.5, margin: "8px 0 0" }}>
              {PROFILE_INFO[profile].blurb}
              {profile === "custom" && " Select categories below — advanced and dangerous categories require explicit opt-in in Settings."}
            </p>
          </div>

          {/* Opportunity list */}
          <div className="stack" style={{ gap: 8 }}>
            {visible.map((o) => {
              const isSel = selected.has(o.ruleId);
              return (
                <div key={o.ruleId} className="card" style={{ padding: 14, opacity: o.blockedReason ? 0.65 : 1 }}>
                  <div className="row-wrap">
                    <input
                      type="checkbox"
                      checked={isSel}
                      disabled={!!o.blockedReason}
                      onChange={() => {
                        setSelected((s) => {
                          const n = new Set(s);
                          if (n.has(o.ruleId)) n.delete(o.ruleId); else n.add(o.ruleId);
                          return n;
                        });
                      }}
                      aria-label={`Include ${o.rule.name}`}
                    />
                    <div style={{ minWidth: 0, flex: 1 }}>
                      <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
                        <b style={{ fontSize: 14 }}>{o.rule.name}</b>
                        <RiskBadge risk={o.rule.risk} />
                        {o.rule.windowsManaged && <span className="badge low">Windows-managed</span>}
                        {o.rule.reversibility === "recyclable" && <span className="badge safe">Recyclable</span>}
                        {o.rule.optIn && <span className="badge review">Opt-in</span>}
                      </div>
                      <div className="dim" style={{ fontSize: 12.5, marginTop: 3 }}>{o.rule.description}</div>
                      {o.blockedReason && (
                        <div className="dim3" style={{ fontSize: 12, marginTop: 3 }}>⚠ {o.blockedReason}</div>
                      )}
                    </div>
                    <div style={{ textAlign: "right" }}>
                      <div className="num" style={{ fontWeight: 650 }}>{formatBytes(o.reclaimableSize)}</div>
                      <div className="dim3 num" style={{ fontSize: 11.5 }}>
                        {formatCount(o.itemCount)} items · {o.confidence}% confidence
                      </div>
                    </div>
                    <button className="btn small ghost" onClick={() => setInspect(o)}>Details</button>
                  </div>
                </div>
              );
            })}
          </div>

          {/* Action bar */}
          <div className="card row">
            <div>
              <div className="dim" style={{ fontSize: 12 }}>Selected reclaim estimate</div>
              <div className="num" style={{ fontSize: 21, fontWeight: 700, color: "var(--accent-2)" }}>
                {formatBytes(selectedTotal)}
              </div>
            </div>
            <div className="spacer" />
            <button className="btn primary" disabled={busy || selected.size === 0} onClick={() => void buildPlan()}>
              Preview cleanup
            </button>
          </div>

          {busy && progress && (
            <div className="card">
              <div className="row">
                <b style={{ fontSize: 13 }}>{progress.rule}</b>
                <div className="spacer" />
                <span className="dim num">{progress.done} / {progress.total}</span>
              </div>
              <div className="progress-line" style={{ marginTop: 8 }}>
                <div style={{ width: progress.total ? `${(progress.done / progress.total) * 100}%` : "5%" }} />
              </div>
            </div>
          )}
        </div>
      )}

      {/* ---- Preview modal (spec §35) ---- */}
      {plan && (
        <Modal
          title="SpaceForge will clean"
          onClose={() => setPlan(null)}
          footer={
            <>
              <button className="btn" onClick={() => setPlan(null)}>Cancel</button>
              <button className="btn" onClick={() => { setPlan(null); }}>Review again</button>
              <button className="btn amber" onClick={() => void execute()} disabled={busy}>
                Clean Now — {formatBytes(plan.estimatedReclaim)}
              </button>
            </>
          }
        >
          {plan.entries.map((e) => (
            <div key={e.ruleId} style={{ padding: "8px 0", borderBottom: "1px solid var(--stroke-1)" }}>
              <div className="row">
                <b>{e.ruleName}</b>
                <RiskBadge risk={e.risk} />
                <span className="badge low">{e.mechanism.replace("_", "-")}</span>
                <div className="spacer" />
                <span className="num">{formatBytes(e.estimatedReclaim)}</span>
              </div>
              <div className="dim" style={{ fontSize: 12.5, marginTop: 2 }}>
                {formatCount(e.itemCount)} items
                {e.targets.length > 0 && ` · ${e.targets.length} location(s)`}
                {e.reversibility === "permanent" && e.mechanism === "direct" && " · permanently deleted"}
                {e.reversibility === "recyclable" && " · restorable via Recycle Bin"}
              </div>
              {e.targets.slice(0, 4).map((t) => (
                <div key={t} className="mono dim3" style={{ fontSize: 11, marginTop: 2, wordBreak: "break-all" }}>{t}</div>
              ))}
              {e.targets.length > 4 && (
                <div className="dim3" style={{ fontSize: 11 }}>… and {e.targets.length - 4} more</div>
              )}
            </div>
          ))}
          <div className="row" style={{ marginTop: 12, fontSize: 15 }}>
            <b>Total estimated reclaim</b>
            <div className="spacer" />
            <b className="num" style={{ color: "var(--accent-2)" }}>{formatBytes(plan.estimatedReclaim)}</b>
          </div>
        </Modal>
      )}

      {/* ---- Report modal (spec §36) ---- */}
      {report && (
        <Modal
          title="Cleanup report"
          onClose={() => setReport(null)}
          footer={<button className="btn" onClick={() => setReport(null)}>Close</button>}
        >
          <div className="grid-3" style={{ marginBottom: 12 }}>
            <div className="kpi amber"><div className="label">Reclaimed</div><div className="value" style={{ fontSize: 18 }}>{formatBytes(report.actualReclaim)}</div></div>
            <div className="kpi"><div className="label">Duration</div><div className="value" style={{ fontSize: 18 }}>{formatDuration(report.durationMs)}</div></div>
            <div className="kpi"><div className="label">Free space</div><div className="value" style={{ fontSize: 18 }}>{formatBytes(report.freeAfter)}</div></div>
          </div>
          {report.entries.map((e) => (
            <div key={e.ruleId} style={{ padding: "6px 0", borderBottom: "1px solid var(--stroke-1)", fontSize: 13 }}>
              <div className="row">
                <b>{e.ruleId}</b>
                <span className={`badge ${e.status === "success" ? "safe" : e.status === "partial" ? "review" : "dangerous"}`}>
                  {e.status}
                </span>
                <div className="spacer" />
                <span className="num">{formatBytes(e.reclaimedBytes)}</span>
              </div>
              {e.failures.map((f, i) => <div key={i} className="dim3" style={{ fontSize: 11.5 }}>failed: {f}</div>)}
              {e.skippedReasons.map((s, i) => <div key={i} className="dim3" style={{ fontSize: 11.5 }}>note: {s}</div>)}
            </div>
          ))}
          <p className="dim3" style={{ fontSize: 12, marginTop: 10 }}>
            Estimated {formatBytes(report.estimatedReclaim)} vs actual {formatBytes(report.actualReclaim)} —
            differences come from locked files and Windows-managed operations.
          </p>
        </Modal>
      )}

      {/* ---- Rule detail modal ---- */}
      {inspect && (
        <Modal
          title={inspect.rule.name}
          onClose={() => setInspect(null)}
          footer={<button className="btn" onClick={() => setInspect(null)}>Close</button>}
        >
          <p style={{ fontSize: 13.5 }}>{inspect.rule.description}</p>
          <div className="stack" style={{ gap: 8, fontSize: 13 }}>
            <div><span className="dim3">Why it is removable: </span>{inspect.rule.reason}</div>
            <div><span className="dim3">Consequence: </span>{inspect.rule.consequence}</div>
            <div><span className="dim3">Reversibility: </span>{inspect.rule.reversibility}</div>
            <div><span className="dim3">Mechanism: </span>{inspect.rule.windowsManaged ? "supported Windows mechanism" : "direct, validated deletion"}</div>
            {inspect.rule.requiresAdmin && <div><span className="dim3">Requires: </span>administrator privileges</div>}
            {inspect.rule.conflictsWithProcesses.length > 0 && (
              <div><span className="dim3">Close before cleaning: </span>{inspect.rule.conflictsWithProcesses.join(", ")}</div>
            )}
            <div><span className="dim3">Locations: </span></div>
            {inspect.locations.map((l) => (
              <div key={l} className="mono dim3" style={{ fontSize: 11, wordBreak: "break-all" }}>{l}</div>
            ))}
          </div>
        </Modal>
      )}
    </div>
  );
}
