/** Windows File Explorer-style vector file and folder icons.
 * Provides authentic, high-dpi, type-specific icons (Folder, Images,
 * Videos, Audio, Code, Executables, Archives, Documents, Databases, etc.)
 */

import React from "react";

interface FileIconProps {
  category?: string;
  extension?: string;
  isDir?: boolean;
  size?: number;
  className?: string;
}

export function FileExplorerIcon({
  category = "unknown",
  extension = "",
  isDir = false,
  size = 20,
  className,
}: FileIconProps) {
  const ext = (extension || "").toLowerCase().replace(/^\./, "");
  const cat = (category || "").toLowerCase();

  // 1. Folder
  if (isDir || cat === "folder") {
    return (
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        className={className}
        style={{ flexShrink: 0 }}
      >
        <path
          d="M2.5 7C2.5 5.9 3.4 5 4.5 5H9L11 7.5H19.5C20.6 7.5 21.5 8.4 21.5 9.5V17.5C21.5 18.6 20.6 19.5 19.5 19.5H4.5C3.4 19.5 2.5 18.6 2.5 17.5V7Z"
          fill="#D97706"
        />
        <path
          d="M2.5 10C2.5 8.9 3.4 8 4.5 8H19.5C20.6 8 21.5 8.9 21.5 10V17.5C21.5 18.6 20.6 19.5 19.5 19.5H4.5C3.4 19.5 2.5 18.6 2.5 17.5V10Z"
          fill="#F59E0B"
        />
        <path
          d="M2.5 12C2.5 10.9 3.4 10 4.5 10H19.5C20.6 10 21.5 10.9 21.5 12V17.5C21.5 18.6 20.6 19.5 19.5 19.5H4.5C3.4 19.5 2.5 18.6 2.5 17.5V12Z"
          fill="#FBBF24"
        />
      </svg>
    );
  }

  // 2. Image
  if (cat === "image" || ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "avif"].includes(ext)) {
    return (
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        className={className}
        style={{ flexShrink: 0 }}
      >
        <rect x="3" y="4" width="18" height="16" rx="3" fill="#EC4899" fillOpacity="0.16" stroke="#EC4899" strokeWidth="1.6" />
        <circle cx="8" cy="9" r="1.8" fill="#EC4899" />
        <path d="M4 17L9 12L13 16L16 13L20 17" stroke="#EC4899" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    );
  }

  // 3. Video
  if (cat === "video" || ["mp4", "mkv", "avi", "mov", "wmv", "webm", "flv"].includes(ext)) {
    return (
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        className={className}
        style={{ flexShrink: 0 }}
      >
        <rect x="3" y="4" width="18" height="16" rx="3" fill="#C026D3" fillOpacity="0.16" stroke="#C026D3" strokeWidth="1.6" />
        <polygon points="10,9 16,12 10,15" fill="#C026D3" />
      </svg>
    );
  }

  // 4. Audio
  if (cat === "audio" || ["mp3", "wav", "flac", "aac", "ogg", "m4a", "wma"].includes(ext)) {
    return (
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        className={className}
        style={{ flexShrink: 0 }}
      >
        <rect x="3" y="4" width="18" height="16" rx="3" fill="#A855F7" fillOpacity="0.16" stroke="#A855F7" strokeWidth="1.6" />
        <path d="M9 15.5A1.5 1.5 0 1 1 7.5 14H9V8L16 6V13.5A1.5 1.5 0 1 1 14.5 12H16V7.5L9 9.5V15.5Z" fill="#A855F7" />
      </svg>
    );
  }

  // 5. Code
  if (cat === "code" || ["js", "ts", "jsx", "tsx", "py", "rs", "go", "cpp", "c", "h", "cs", "java", "php", "html", "css", "json", "yml", "yaml", "toml", "sql"].includes(ext)) {
    return (
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        className={className}
        style={{ flexShrink: 0 }}
      >
        <path d="M4 4C4 2.9 4.9 2 6 2H14L20 8V20C20 21.1 19.1 22 18 22H6C4.9 22 4 21.1 4 20V4Z" fill="#06B6D4" fillOpacity="0.14" stroke="#06B6D4" strokeWidth="1.6" />
        <polyline points="14 2 14 8 20 8" stroke="#06B6D4" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
        <path d="M9 13L7.5 14.5L9 16M13 13L14.5 14.5L13 16M11.5 12.5L10.5 16.5" stroke="#06B6D4" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    );
  }

  // 6. Archive / Compressed
  if (cat === "archive" || ["zip", "rar", "7z", "tar", "gz", "bz2", "xz", "iso", "cab"].includes(ext)) {
    return (
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        className={className}
        style={{ flexShrink: 0 }}
      >
        <path d="M4 4C4 2.9 4.9 2 6 2H14L20 8V20C20 21.1 19.1 22 18 22H6C4.9 22 4 21.1 4 20V4Z" fill="#EAB308" fillOpacity="0.14" stroke="#EAB308" strokeWidth="1.6" />
        <polyline points="14 2 14 8 20 8" stroke="#EAB308" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
        <rect x="9.5" y="11" width="3" height="7" rx="1" fill="#EAB308" />
        <line x1="11" y1="5" x2="11" y2="11" stroke="#EAB308" strokeWidth="1.5" strokeDasharray="1.5 1.5" />
      </svg>
    );
  }

  // 7. Executable / Application
  if (cat === "executable" || ["exe", "msi", "bat", "cmd", "ps1", "vbs", "com"].includes(ext)) {
    return (
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        className={className}
        style={{ flexShrink: 0 }}
      >
        <rect x="3" y="4" width="18" height="16" rx="3" fill="#10B981" fillOpacity="0.16" stroke="#10B981" strokeWidth="1.6" />
        <line x1="3" y1="8.5" x2="21" y2="8.5" stroke="#10B981" strokeWidth="1.4" />
        <circle cx="6" cy="6.2" r="1" fill="#10B981" />
        <circle cx="9" cy="6.2" r="1" fill="#10B981" />
        <polygon points="10,12 15,15 10,18" fill="#10B981" />
      </svg>
    );
  }

  // 8. Database
  if (cat === "database" || ["db", "sqlite", "sqlite3", "mdb", "accdb", "frm", "ibd"].includes(ext)) {
    return (
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        className={className}
        style={{ flexShrink: 0 }}
      >
        <ellipse cx="12" cy="6" rx="8" ry="3" fill="#8B5CF6" fillOpacity="0.2" stroke="#8B5CF6" strokeWidth="1.6" />
        <path d="M4 6V12C4 13.7 7.6 15 12 15C16.4 15 20 13.7 20 12V6" stroke="#8B5CF6" strokeWidth="1.6" />
        <path d="M4 12V18C4 19.7 7.6 21 12 21C16.4 21 20 19.7 20 18V12" stroke="#8B5CF6" strokeWidth="1.6" />
      </svg>
    );
  }

  // 9. Document / Text
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      className={className}
      style={{ flexShrink: 0 }}
    >
      <path
        d="M4 4C4 2.9 4.9 2 6 2H14L20 8V20C20 21.1 19.1 22 18 22H6C4.9 22 4 21.1 4 20V4Z"
        fill="#3B82F6"
        fillOpacity="0.12"
        stroke="#3B82F6"
        strokeWidth="1.6"
      />
      <polyline points="14 2 14 8 20 8" stroke="#3B82F6" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
      <line x1="8" y1="13" x2="16" y2="13" stroke="#3B82F6" strokeWidth="1.5" strokeLinecap="round" />
      <line x1="8" y1="17" x2="13" y2="17" stroke="#3B82F6" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}
