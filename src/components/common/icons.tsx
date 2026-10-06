/** Inline SVG icon set — no runtime dependency, tuned for 16–18px. */

import React from "react";

interface P {
  size?: number;
  className?: string;
}

function svg(path: React.ReactNode, viewBox = "0 0 24 24") {
  return ({ size = 17, className }: P) => (
    <svg
      width={size}
      height={size}
      viewBox={viewBox}
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      {path}
    </svg>
  );
}

export const IconDashboard = svg(<>
  <rect x="3" y="3" width="7" height="9" rx="1.5" />
  <rect x="14" y="3" width="7" height="5" rx="1.5" />
  <rect x="14" y="12" width="7" height="9" rx="1.5" />
  <rect x="3" y="16" width="7" height="5" rx="1.5" />
</>);

export const IconAnalyzer = svg(<>
  <path d="M4 20v-6M9 20V8M14 20v-9M19 20V4" />
</>);

export const IconTreemap = svg(<>
  <rect x="3" y="3" width="11" height="11" rx="1" />
  <rect x="16" y="3" width="5" height="7" rx="1" />
  <rect x="16" y="12" width="5" height="9" rx="1" />
  <rect x="3" y="16" width="11" height="5" rx="1" />
</>);

export const IconLargeFiles = svg(<>
  <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
  <path d="M14 3v5h5" />
  <path d="M9 15l6-6M9.5 9.5h.01M14.5 14.5h.01" />
</>);

export const IconFileTypes = svg(<>
  <circle cx="12" cy="12" r="9" />
  <path d="M12 3a9 9 0 0 1 9 9h-9z" fill="currentColor" stroke="none" />
</>);

export const IconDuplicates = svg(<>
  <rect x="8" y="8" width="12" height="12" rx="2" />
  <path d="M16 4H6a2 2 0 0 0-2 2v10" />
</>);

export const IconCleanup = svg(<>
  <path d="M4 6h16M9 6V4h6v2M6 6l1 14h10l1-14" />
  <path d="M10 11v6M14 11v6" />
</>);

export const IconWindows = svg(<>
  <rect x="3" y="3" width="8" height="8" rx="1" />
  <rect x="13" y="3" width="8" height="8" rx="1" />
  <rect x="3" y="13" width="8" height="8" rx="1" />
  <rect x="13" y="13" width="8" height="8" rx="1" />
</>);

export const IconApps = svg(<>
  <rect x="3" y="7" width="18" height="13" rx="2" />
  <path d="M7 7V5a2 2 0 0 1 2-2h6a2 2 0 0 1 2 2v2" />
</>);

export const IconGlobe = svg(<>
  <circle cx="12" cy="12" r="9" />
  <path d="M3 12h18M12 3a15 15 0 0 1 0 18 15 15 0 0 1 0-18z" />
</>);

export const IconCode = svg(<>
  <path d="M8 9l-4 3 4 3M16 9l4 3-4 3M13 5l-2 14" />
</>);

export const IconClock = svg(<>
  <circle cx="12" cy="12" r="9" />
  <path d="M12 7v5l3 3" />
</>);

export const IconHistory = svg(<>
  <path d="M3 12a9 9 0 1 0 3-6.7L3 8" />
  <path d="M3 3v5h5M12 7v5l4 2" />
</>);

export const IconInsights = svg(<>
  <path d="M12 3a6 6 0 0 1 6 6c0 2.5-1.5 4-2.5 5.5-.7 1-.5 2.5-.5 2.5h-6s.2-1.5-.5-2.5C7.5 13 6 11.5 6 9a6 6 0 0 1 6-6z" />
  <path d="M10 21h4" />
</>);

export const IconQuestion = svg(<>
  <circle cx="12" cy="12" r="9" />
  <path d="M9.5 9a2.5 2.5 0 1 1 3.8 2.1c-.8.5-1.3 1-1.3 1.9" />
  <path d="M12 17h.01" />
</>);

export const IconAdvanced = svg(<>
  <circle cx="12" cy="12" r="3" />
  <path d="M19 12a7 7 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14 3h-4l-.5 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6a7 7 0 0 0 0 2.4l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2L10 21h4l.5-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.07-.4.1-.8.1-1.2z" />
</>);

export const IconActivity = svg(<>
  <path d="M3 12h4l2-7 4 14 2-7h6" />
</>);

export const IconSettings = svg(<>
  <circle cx="12" cy="12" r="3" />
  <path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1" />
</>);

export const IconSearch = svg(<>
  <circle cx="11" cy="11" r="7" />
  <path d="M21 21l-4.3-4.3" />
</>);

export const IconFolder = svg(<>
  <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
</>);

export const IconFile = svg(<>
  <path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
  <path d="M14 3v5h5" />
</>);

export const IconChevronRight = svg(<path d="M9 6l6 6-6 6" />);
export const IconChevronLeft = svg(<path d="M15 6l-6 6 6 6" />);
export const IconClose = svg(<path d="M6 6l12 12M18 6L6 18" />);
export const IconPlay = svg(<path d="M7 4l13 8-13 8z" />);
export const IconPause = svg(<>
  <rect x="6" y="4" width="4" height="16" rx="1" />
  <rect x="14" y="4" width="4" height="16" rx="1" />
</>);
export const IconStop = svg(<rect x="6" y="6" width="12" height="12" rx="1.5" />);
export const IconTrash = svg(<>
  <path d="M4 6h16M9 6V4h6v2M6 6l1 14h10l1-14" />
</>);
export const IconShield = svg(<>
  <path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z" />
  <path d="M9 12l2 2 4-4" />
</>);
export const IconCamera = svg(<>
  <path d="M4 8h3l2-3h6l2 3h3a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1z" />
  <circle cx="12" cy="13" r="4" />
</>);
export const IconDrive = svg(<>
  <rect x="3" y="8" width="18" height="8" rx="2" />
  <path d="M6 19h12M7 12h.01M10 12h.01" />
</>);

export const IconArrowUp = svg(<path d="M12 19V5M5 12l7-7 7 7" />);
export const IconExternalLink = svg(<>
  <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
  <polyline points="15 3 21 3 21 9" />
  <line x1="10" y1="14" x2="21" y2="3" />
</>);
export const IconCopy = svg(<>
  <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
</>);
export const IconViewList = svg(<>
  <line x1="8" y1="6" x2="21" y2="6" />
  <line x1="8" y1="12" x2="21" y2="12" />
  <line x1="8" y1="18" x2="21" y2="18" />
  <circle cx="4" cy="6" r="1.5" fill="currentColor" stroke="none" />
  <circle cx="4" cy="12" r="1.5" fill="currentColor" stroke="none" />
  <circle cx="4" cy="18" r="1.5" fill="currentColor" stroke="none" />
</>);
export const IconViewGrid = svg(<>
  <rect x="3" y="3" width="7" height="7" rx="1.5" />
  <rect x="14" y="3" width="7" height="7" rx="1.5" />
  <rect x="14" y="14" width="7" height="7" rx="1.5" />
  <rect x="3" y="14" width="7" height="7" rx="1.5" />
</>);

