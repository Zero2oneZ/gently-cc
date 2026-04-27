// gently-cc shared theme utility
// Deterministic color identity per project — the visual GPS layer.
// Each project gets one of 16 named themes derived from its project_id hash.
// Theme name = the project's "call sign" in multi-agent / buddies sessions.

export const THEMES = [
  { name: 'Amber',   primary: '\x1b[38;5;214m', accent: '\x1b[38;5;220m', dim: '\x1b[38;5;130m' },
  { name: 'Cobalt',  primary: '\x1b[38;5;69m',  accent: '\x1b[38;5;75m',  dim: '\x1b[38;5;25m'  },
  { name: 'Moss',    primary: '\x1b[38;5;107m', accent: '\x1b[38;5;113m', dim: '\x1b[38;5;65m'  },
  { name: 'Crimson', primary: '\x1b[38;5;160m', accent: '\x1b[38;5;196m', dim: '\x1b[38;5;88m'  },
  { name: 'Slate',   primary: '\x1b[38;5;103m', accent: '\x1b[38;5;147m', dim: '\x1b[38;5;60m'  },
  { name: 'Ember',   primary: '\x1b[38;5;202m', accent: '\x1b[38;5;208m', dim: '\x1b[38;5;130m' },
  { name: 'Jade',    primary: '\x1b[38;5;78m',  accent: '\x1b[38;5;84m',  dim: '\x1b[38;5;36m'  },
  { name: 'Violet',  primary: '\x1b[38;5;135m', accent: '\x1b[38;5;141m', dim: '\x1b[38;5;91m'  },
  { name: 'Rust',    primary: '\x1b[38;5;166m', accent: '\x1b[38;5;172m', dim: '\x1b[38;5;94m'  },
  { name: 'Steel',   primary: '\x1b[38;5;109m', accent: '\x1b[38;5;153m', dim: '\x1b[38;5;66m'  },
  { name: 'Sage',    primary: '\x1b[38;5;114m', accent: '\x1b[38;5;120m', dim: '\x1b[38;5;71m'  },
  { name: 'Gold',    primary: '\x1b[38;5;178m', accent: '\x1b[38;5;184m', dim: '\x1b[38;5;136m' },
  { name: 'Navy',    primary: '\x1b[38;5;26m',  accent: '\x1b[38;5;69m',  dim: '\x1b[38;5;17m'  },
  { name: 'Coral',   primary: '\x1b[38;5;209m', accent: '\x1b[38;5;215m', dim: '\x1b[38;5;167m' },
  { name: 'Teal',    primary: '\x1b[38;5;37m',  accent: '\x1b[38;5;80m',  dim: '\x1b[38;5;23m'  },
  { name: 'Bronze',  primary: '\x1b[38;5;136m', accent: '\x1b[38;5;179m', dim: '\x1b[38;5;94m'  },
];

const RESET = '\x1b[0m';
const BOLD  = '\x1b[1m';

export function themeForIdentity(identity) {
  // Theme is stored in identity.json after install — deterministic fallback from project_id
  if (identity.theme) {
    const t = THEMES.find(t => t.name === identity.theme);
    if (t) return t;
  }
  const id = identity.project_id || identity.project || 'default';
  const byte = parseInt(id.slice(0, 2), 16) || 0;
  return THEMES[byte % 16];
}

export function themeIndexForProject(projectId) {
  const byte = parseInt((projectId || '').slice(0, 2), 16) || 0;
  return byte % 16;
}

export function c(theme, variant, text) {
  return `${theme[variant] || ''}${text}${RESET}`;
}

// The ASCII character — colored with the project theme.
// Three lines, pre-padded for alignment beside info text.
export function asciiArt(theme) {
  const p = theme.primary;
  const a = theme.accent;
  const d = theme.dim;
  return [
    ` ${p}▐▛███▜▌${RESET}`,
    `${a}▝▜█████▛▘${RESET}`,
    `${d}  ▘▘ ▝▝${RESET}  `,
  ];
}

// Compact GPS tag for the stats line — [CRIMSON] Greg/gently-cc
export function gpsTag(theme, handle, project) {
  return `${BOLD}${theme.accent}[${theme.name.toUpperCase()}]${RESET} ${theme.dim}${handle}/${project}${RESET}`;
}

// Full session banner — art left, info right
export function sessionBanner(identity, session, opts = {}) {
  const theme = themeForIdentity(identity);
  const art = asciiArt(theme);
  const p = theme.primary;
  const a = theme.accent;
  const d = theme.dim;

  const themeBadge  = `${BOLD}${a}[${theme.name.toUpperCase()}]${RESET}`;
  const nameStr     = `${BOLD}${p}${identity.name}${RESET}`;
  const projectStr  = `${a}${identity.project}${RESET}`;
  const stackStr    = `${d}CODIE · BARF · CODEC${RESET}`;
  const pathStr     = `${d}${opts.projectPath || identity.project_path || '~'}${RESET}`;
  const sessionStr  = `${d}${(session.session_id || '').slice(0, 16)}${RESET}`;
  const parentStr   = session.crystal_parent
    ? `${d}↑ ${session.crystal_parent.slice(0, 20)}…${RESET}`
    : `${d}(new branch)${RESET}`;

  const lines = [
    `${art[0]}  ${themeBadge}  ${nameStr}`,
    `${art[1]}  ${projectStr}  ${stackStr}`,
    `${art[2]}  ${pathStr}`,
    `    ${d}session ${sessionStr}  ${parentStr}${RESET}`,
  ];

  return lines.join('\n');
}
