/** Disk Analyzer — Windows File Explorer-style tree browser (spec §14, §15). */

import React, { useEffect, useMemo, useState } from "react";
import { ipc } from "../api/ipc";
import type { DirNode } from "../api/types";
import { formatBytes, formatCount, ageFrom, formatDateTime } from "../lib/format";
import { colorFor, labelFor } from "../lib/categoryColor";
import { useApp } from "../state/AppState";
import {
  Column, ContextMenu, CtxMenuItem, EmptyState, usePathActions, VirtualTable,
} from "../components/common/UI";
import { FileExplorerIcon } from "../components/common/FileExplorerIcon";
import {
  IconArrowUp, IconChevronRight, IconClose, IconCopy, IconDrive,
  IconExternalLink, IconSearch, IconTrash, IconViewGrid, IconViewList,
} from "../components/common/icons";

interface Row {
  name: string;
  path: string;
  kind: "dir" | "file" | "aggregate";
  size: number;
  fileCount: number;
  modified: string | null;
  category: string;
  extension: string;
}

function getFileTypeLabel(kind: string, ext: string, cat: string): string {
  if (kind === "dir") return "File folder";
  if (kind === "aggregate") return "File group";
  const e = ext.toLowerCase();
  if (["png", "jpg", "jpeg", "webp", "gif"].includes(e)) return `${e.toUpperCase()} Image`;
  if (["mp4", "mkv", "avi", "mov"].includes(e)) return `${e.toUpperCase()} Video`;
  if (["mp3", "flac", "wav", "aac"].includes(e)) return `${e.toUpperCase()} Audio`;
  if (["zip", "rar", "7z", "tar", "gz"].includes(e)) return `${e.toUpperCase()} Archive`;
  if (["exe", "msi"].includes(e)) return "Application";
  if (["dll", "sys"].includes(e)) return "System file";
  if (["ts", "tsx", "js", "jsx"].includes(e)) return "TypeScript/JavaScript";
  if (["rs", "py", "c", "cpp", "cs", "go"].includes(e)) return `${e.toUpperCase()} Source Code`;
  if (["pdf"].includes(e)) return "PDF Document";
  if (["txt", "md", "log"].includes(e)) return "Text Document";
  if (e) return `${e.toUpperCase()} File`;
  return labelFor(cat);
}

export function AnalyzerView() {
  const { lastScan, scanning } = useApp();
  const [path, setPath] = useState("");
  const [node, setNode] = useState<DirNode | null>(null);
  const [selectedRow, setSelectedRow] = useState<Row | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; row: Row } | null>(null);
  const [searchFilter, setSearchFilter] = useState("");
  const [viewMode, setViewMode] = useState<"details" | "tiles">("details");
  const [showInspector, setShowInspector] = useState(true);

  const pathActions = usePathActions();

  useEffect(() => {
    let alive = true;
    ipc.drillDown(path).then((n) => {
      if (alive) {
        setNode(n);
        setSelectedRow(null);
      }
    }).catch(() => alive && setNode(null));
    return () => { alive = false; };
  }, [path, lastScan]);

  const allRows: Row[] = useMemo(() => {
    if (!node) return [];
    const list: Row[] = [
      ...node.children.map<Row>((c) => ({
        name: c.name,
        path: c.path,
        kind: "dir",
        size: c.size,
        fileCount: c.fileCount,
        modified: c.modified,
        category: "folder",
        extension: "",
      })),
      ...node.files.map<Row>((f) => {
        const lastDot = f.name.lastIndexOf(".");
        const ext = lastDot > 0 ? f.name.slice(lastDot + 1) : "";
        return {
          name: f.name,
          path: f.path,
          kind: "file",
          size: f.size,
          fileCount: 1,
          modified: f.modified,
          category: f.category,
          extension: ext,
        };
      }),
    ];
    if (node.smallFilesSize > 0) {
      list.push({
        name: `Many small files (${formatCount(node.smallFilesCount)})`,
        path: node.path,
        kind: "aggregate",
        size: node.smallFilesSize,
        fileCount: node.smallFilesCount,
        modified: null,
        category: "unknown",
        extension: "",
      });
    }
    return list;
  }, [node]);

  const filteredRows = useMemo(() => {
    if (!searchFilter.trim()) return allRows;
    const q = searchFilter.toLowerCase();
    return allRows.filter((r) => r.name.toLowerCase().includes(q) || r.extension.toLowerCase().includes(q));
  }, [allRows, searchFilter]);

  if (scanning) return <EmptyState title="Scanning in progress…" hint="Disk analysis unlocks when the scan completes." />;
  if (!lastScan) return <EmptyState title="No scan available" hint="Scan a volume from the Dashboard first to inspect files." />;
  if (!node) return <EmptyState title="Nothing to show" hint="This path has no analyzable content in the current scan." />;

  const parts = path.split(/[\\/]/).filter(Boolean);

  const goUp = () => {
    if (parts.length <= 1) {
      setPath("");
    } else {
      setPath(parts.slice(0, -1).join("/"));
    }
  };

  const cols: Column<Row>[] = [
    {
      key: "name",
      label: "Name",
      width: "minmax(280px, 1.4fr)",
      sortValue: (r) => r.name.toLowerCase(),
      render: (r) => (
        <span
          className="row"
          style={{ gap: 9, cursor: r.kind === "dir" ? "pointer" : "default" }}
          onClick={() => setSelectedRow(r)}
          onDoubleClick={() => r.kind === "dir" && setPath(r.path)}
        >
          <FileExplorerIcon
            category={r.category}
            extension={r.extension}
            isDir={r.kind === "dir"}
            size={18}
          />
          <span
            style={{
              overflow: "hidden",
              textOverflow: "ellipsis",
              fontWeight: r.kind === "dir" ? 600 : 400,
            }}
            title={r.path}
          >
            {r.name}
          </span>
        </span>
      ),
    },
    {
      key: "type",
      label: "Type",
      width: "140px",
      sortValue: (r) => getFileTypeLabel(r.kind, r.extension, r.category),
      render: (r) => (
        <span className="dim" style={{ fontSize: 12 }}>
          {getFileTypeLabel(r.kind, r.extension, r.category)}
        </span>
      ),
    },
    {
      key: "size",
      label: "Size",
      width: "105px",
      align: "right",
      sortValue: (r) => r.size,
      render: (r) => <span className="num" style={{ fontWeight: 600 }}>{formatBytes(r.size)}</span>,
    },
    {
      key: "pct",
      label: "% of folder",
      width: "120px",
      align: "right",
      sortValue: (r) => (node.size ? r.size / node.size : 0),
      render: (r) => (
        <span className="row" style={{ justifyContent: "flex-end", gap: 7 }}>
          <span className="bar-track" style={{ width: 48 }}>
            <span
              className="bar-fill"
              style={{
                width: `${node.size ? Math.min(100, (r.size / node.size) * 100) : 0}%`,
                display: "block",
              }}
            />
          </span>
          <span className="num dim" style={{ fontSize: 11.5 }}>
            {node.size ? ((r.size / node.size) * 100).toFixed(1) : "0"}%
          </span>
        </span>
      ),
    },
    {
      key: "files",
      label: "Items",
      width: "80px",
      align: "right",
      sortValue: (r) => r.fileCount,
      render: (r) => (
        <span className="num dim">
          {r.kind === "dir" ? formatCount(r.fileCount) : "—"}
        </span>
      ),
    },
    {
      key: "modified",
      label: "Date modified",
      width: "120px",
      sortValue: (r) => r.modified ?? "",
      render: (r) => (
        <span className="dim" title={r.modified ? formatDateTime(r.modified) : ""}>
          {ageFrom(r.modified)}
        </span>
      ),
    },
  ];

  return (
    <div style={{ height: "calc(100vh - 120px)", display: "flex", flexDirection: "column" }}>
      {/* Title & Stats */}
      <div className="row-wrap" style={{ marginBottom: 4 }}>
        <h1 className="page-title" style={{ margin: 0 }}>Disk Analyzer</h1>
        <div className="spacer" />
        <span className="cosmic-chip">
          <span className="chip-dot" />
          {formatCount(filteredRows.length)} items · {formatBytes(node.size)} total
        </span>
      </div>

      <p className="page-sub" style={{ marginBottom: 10 }}>
        Windows File Explorer navigation. Double-click folders to drill down, right-click for file actions.
      </p>

      {/* Windows File Explorer Address Bar & Action Toolbar */}
      <div className="explorer-toolbar">
        {/* Up to Parent Button */}
        <button
          className="btn ghost small"
          onClick={goUp}
          disabled={!path}
          title="Up to parent folder (Alt+Up)"
          style={{ padding: "4px 8px" }}
        >
          <IconArrowUp size={15} />
        </button>

        {/* Address Bar Breadcrumbs */}
        <div className="explorer-address-bar">
          <IconDrive size={15} className="dim" />
          <button className="explorer-address-crumb" onClick={() => setPath("")}>
            {lastScan.volume}
          </button>
          {parts.map((p, i) => (
            <React.Fragment key={p + i}>
              <span className="sep dim3" style={{ fontSize: 11 }}>›</span>
              <button
                className="explorer-address-crumb"
                onClick={() => setPath(parts.slice(0, i + 1).join("/"))}
                style={{ fontWeight: i === parts.length - 1 ? 600 : 400 }}
              >
                {p}
              </button>
            </React.Fragment>
          ))}
          <div className="spacer" />
          <button
            className="btn ghost small"
            onClick={() => void pathActions(node.path, "copy")}
            title="Copy current folder path"
            style={{ padding: "2px 6px" }}
          >
            <IconCopy size={13} />
          </button>
          <button
            className="btn ghost small"
            onClick={() => void pathActions(node.path, "folder")}
            title="Open folder in Windows Explorer"
            style={{ padding: "2px 6px" }}
          >
            <IconExternalLink size={13} />
          </button>
        </div>

        {/* Search / Filter input */}
        <div className="row" style={{ background: "var(--bg-s2)", border: "1px solid var(--stroke-2)", borderRadius: "var(--radius-s)", padding: "2px 8px" }}>
          <IconSearch size={13} className="dim3" />
          <input
            type="text"
            placeholder="Search folder…"
            value={searchFilter}
            onChange={(e) => setSearchFilter(e.target.value)}
            style={{ border: "none", background: "transparent", padding: "3px 6px", fontSize: 12.5, width: 140 }}
          />
          {searchFilter && (
            <button className="btn ghost small" onClick={() => setSearchFilter("")} style={{ padding: 2 }}>
              <IconClose size={11} />
            </button>
          )}
        </div>

        {/* View Mode Switcher */}
        <div className="seg">
          <button
            className={viewMode === "details" ? "active" : ""}
            onClick={() => setViewMode("details")}
            title="Details Table View"
          >
            <IconViewList size={14} />
          </button>
          <button
            className={viewMode === "tiles" ? "active" : ""}
            onClick={() => setViewMode("tiles")}
            title="Tiles Grid View"
          >
            <IconViewGrid size={14} />
          </button>
        </div>

        {/* Toggle Inspector Button */}
        <button
          className={`btn small ${showInspector ? "primary" : "ghost"}`}
          onClick={() => setShowInspector(!showInspector)}
          title="Toggle Details Inspector Pane"
        >
          Details
        </button>
      </div>

      {/* Main Content Area: Table / Grid + Inspector Pane */}
      <div style={{ flex: 1, minHeight: 0, display: "flex", gap: 12 }}>
        {/* Left: Files View */}
        <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
          {viewMode === "details" ? (
            <VirtualTable
              rows={filteredRows}
              columns={cols}
              defaultSort="size"
              onRowActivate={(r) => {
                setSelectedRow(r);
                if (r.kind === "dir") setPath(r.path);
              }}
              onRowContext={(row, x, y) => {
                setSelectedRow(row);
                setMenu({ x, y, row });
              }}
            />
          ) : (
            <div className="explorer-tile-grid">
              {filteredRows.map((r) => {
                const isSelected = selectedRow?.path === r.path;
                return (
                  <div
                    key={r.path}
                    className={`explorer-tile ${isSelected ? "selected" : ""}`}
                    onClick={() => setSelectedRow(r)}
                    onDoubleClick={() => r.kind === "dir" && setPath(r.path)}
                    onContextMenu={(e) => {
                      e.preventDefault();
                      setSelectedRow(r);
                      setMenu({ x: e.clientX, y: e.clientY, row: r });
                    }}
                  >
                    <FileExplorerIcon
                      category={r.category}
                      extension={r.extension}
                      isDir={r.kind === "dir"}
                      size={28}
                    />
                    <div style={{ minWidth: 0, flex: 1 }}>
                      <div
                        style={{
                          fontWeight: r.kind === "dir" ? 600 : 500,
                          fontSize: 13,
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          whiteSpace: "nowrap",
                        }}
                        title={r.name}
                      >
                        {r.name}
                      </div>
                      <div className="dim num" style={{ fontSize: 11.5 }}>
                        {formatBytes(r.size)}
                        {r.kind === "dir" ? ` · ${formatCount(r.fileCount)} files` : ""}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Right: Windows Explorer Details & Properties Inspector Pane */}
        {showInspector && (
          <div className="explorer-inspector">
            {selectedRow ? (
              <>
                <div className="row" style={{ alignItems: "flex-start", gap: 12 }}>
                  <FileExplorerIcon
                    category={selectedRow.category}
                    extension={selectedRow.extension}
                    isDir={selectedRow.kind === "dir"}
                    size={42}
                  />
                  <div style={{ minWidth: 0, flex: 1 }}>
                    <div
                      style={{
                        fontWeight: 700,
                        fontSize: 14,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        wordBreak: "break-all",
                      }}
                    >
                      {selectedRow.name}
                    </div>
                    <span className="badge low" style={{ marginTop: 4, fontSize: 10.5 }}>
                      {getFileTypeLabel(selectedRow.kind, selectedRow.extension, selectedRow.category)}
                    </span>
                  </div>
                </div>

                <div className="divider" style={{ margin: "4px 0" }} />

                <div className="stack" style={{ gap: 8, fontSize: 12 }}>
                  <div>
                    <span className="dim3">Size</span>
                    <div className="num" style={{ fontWeight: 650, fontSize: 14 }}>
                      {formatBytes(selectedRow.size)}
                      <span className="dim" style={{ fontSize: 11, marginLeft: 6, fontWeight: 400 }}>
                        ({selectedRow.size.toLocaleString()} bytes)
                      </span>
                    </div>
                  </div>

                  {selectedRow.kind === "dir" && (
                    <div>
                      <span className="dim3">Contains</span>
                      <div className="num" style={{ fontWeight: 600 }}>
                        {formatCount(selectedRow.fileCount)} items
                      </div>
                    </div>
                  )}

                  <div>
                    <span className="dim3">Modified</span>
                    <div style={{ fontWeight: 500 }}>
                      {selectedRow.modified ? formatDateTime(selectedRow.modified) : "—"}
                    </div>
                  </div>

                  <div>
                    <span className="dim3">Location</span>
                    <div
                      className="mono dim"
                      style={{
                        fontSize: 11,
                        background: "var(--bg-s2)",
                        padding: "6px 8px",
                        borderRadius: "var(--radius-s)",
                        border: "1px solid var(--stroke-2)",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        wordBreak: "break-all",
                        userSelect: "text",
                      }}
                      title={selectedRow.path}
                    >
                      {selectedRow.path}
                    </div>
                  </div>
                </div>

                <div className="spacer" />

                {/* Quick Actions */}
                <div className="stack" style={{ gap: 6 }}>
                  {selectedRow.kind === "dir" ? (
                    <button
                      className="btn primary small"
                      style={{ width: "100%", justifyContent: "center" }}
                      onClick={() => setPath(selectedRow.path)}
                    >
                      Explore Folder <IconChevronRight size={13} />
                    </button>
                  ) : (
                    <button
                      className="btn primary small"
                      style={{ width: "100%", justifyContent: "center" }}
                      onClick={() => void pathActions(selectedRow.path, "open")}
                    >
                      Open File
                    </button>
                  )}

                  <button
                    className="btn small"
                    style={{ width: "100%", justifyContent: "center" }}
                    onClick={() => void pathActions(selectedRow.path, "folder")}
                  >
                    <IconExternalLink size={13} /> Show in Windows Explorer
                  </button>

                  <button
                    className="btn small"
                    style={{ width: "100%", justifyContent: "center" }}
                    onClick={() => void pathActions(selectedRow.path, "copy")}
                  >
                    <IconCopy size={13} /> Copy Full Path
                  </button>

                  <button
                    className="btn small danger"
                    style={{ width: "100%", justifyContent: "center" }}
                    onClick={() => void pathActions(selectedRow.path, "recycle")}
                  >
                    <IconTrash size={13} /> Send to Recycle Bin
                  </button>
                </div>
              </>
            ) : (
              <div
                style={{
                  height: "100%",
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "center",
                  justifyContent: "center",
                  color: "var(--text-3)",
                  textAlign: "center",
                  padding: 12,
                }}
              >
                <div style={{ opacity: 0.3, marginBottom: 8 }}>
                  <IconDrive size={32} />
                </div>
                <div style={{ fontSize: 13, fontWeight: 600 }}>Select a file or folder</div>
                <div style={{ fontSize: 11.5, marginTop: 4 }}>
                  Click any row to preview its metadata, exact size, and quick Explorer actions.
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Context Menu */}
      {menu && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          onClose={() => setMenu(null)}
          items={[
            ...(menu.row.kind === "dir"
              ? [{ label: "Explore this folder", action: () => setPath(menu.row.path) }]
              : [{ label: "Open file", action: () => void pathActions(menu.row.path, "open") }]),
            { label: "Show in Windows Explorer", action: () => void pathActions(menu.row.path, "folder") },
            { label: "Copy full path", action: () => void pathActions(menu.row.path, "copy") },
            { label: "-" } as unknown as CtxMenuItem,
            { label: "Move to Recycle Bin", danger: true, action: () => void pathActions(menu.row.path, "recycle") },
          ]}
        />
      )}

      {/* Access Denied / Unreadable Warning */}
      {node.unreadable.length > 0 && (
        <div className="warning-box" style={{ marginTop: 10 }}>
          {node.unreadable.length} entries could not be read here (access denied or vanished
          during scan). Sizes may be incomplete. Restarting SpaceForge as administrator can
          reveal protected system items.
        </div>
      )}
    </div>
  );
}
