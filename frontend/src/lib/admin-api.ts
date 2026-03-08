export type AdminConfigResponse = {
  status: string;
  config: {
    auth_host: string;
    auth_port: number;
    base_host: string;
    base_port: number;
    cell_host: string;
    cell_port: number;
    admin_port: number;
    developer_mode: boolean;
  };
};

export type AdminStatusResponse = {
  status: string;
  uptime_seconds: number;
  services: {
    auth: boolean;
    base: boolean;
    cell: boolean;
    database: boolean;
  };
};

export type PlayerRecord = {
  id: number;
  name: string;
  archetype: string;
  level: number;
  zone: string;
  ping: number | null;
  status: string;
  session: string;
};

export type PlayersResponse = {
  status: string;
  available: boolean;
  reason: string | null;
  players: PlayerRecord[];
  summary: {
    online_count: number;
    ready: boolean;
  };
  services: AdminStatusResponse['services'];
};

export type SpaceRecord = {
  world_id: number;
  world: string;
  client_map: string;
  has_script: boolean;
  flags: number;
  mission_count: number;
};

export type SpacesResponse = {
  status: string;
  available: boolean;
  reason: string | null;
  spaces: SpaceRecord[];
  summary: {
    total_spaces: number;
    scripted_spaces: number;
    mission_links: number;
  };
};

export type ContentSummaryResponse = {
  status: string;
  available: boolean;
  reason: string | null;
  summary: {
    world_count: number;
    scripted_world_count: number;
    mission_count: number;
    story_mission_count: number;
    hidden_mission_count: number;
    scripted_mission_count: number;
  };
  top_space_mission_counts: Array<{
    scope: string;
    mission_count: number;
  }>;
};

export type ContentEditorPickerOption = {
  value: string;
  label: string;
  group?: string;
};

export type ContentEditorMissionOption = ContentEditorPickerOption & {
  space_id: string;
};

export type ContentEditorRegionOption = ContentEditorPickerOption & {
  space_id: string;
};

export type ContentEditorStepOption = ContentEditorPickerOption & {
  mission_id: string;
};

export type ContentEditorPickersResponse = {
  status: string;
  available: boolean;
  reason: string | null;
  spaces: ContentEditorPickerOption[];
  missions: ContentEditorMissionOption[];
  dialogs: ContentEditorPickerOption[];
  items: ContentEditorPickerOption[];
  regions: ContentEditorRegionOption[];
  steps: ContentEditorStepOption[];
};

export type DashboardStat = {
  label: string;
  value: string;
  detail: string;
};

export type DashboardActivity = {
  time: string;
  title: string;
  detail: string;
};

export type ServiceHealthCard = {
  name: string;
  status: string;
  detail: string;
  variant: 'success' | 'outline' | 'destructive';
};

function getAdminApiOrigin() {
  if (typeof window !== 'undefined' && window.__TAURI_INTERNALS__) {
    return 'http://127.0.0.1:8443';
  }

  // In browser dev/prod, use relative URLs so Vite proxy (dev) or
  // same-origin deployment (prod) handles routing to the backend.
  return '';
}

const ADMIN_API_ORIGIN =
  import.meta.env.VITE_ADMIN_API_ORIGIN ?? getAdminApiOrigin();

export class ConnectionError extends Error {
  constructor(path: string, cause?: unknown) {
    super(`Server unreachable while fetching ${path}`);
    this.name = 'ConnectionError';
    this.cause = cause;
  }
}

async function fetchAdminJson<T>(path: string): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`${ADMIN_API_ORIGIN}${path}`, {
      headers: {
        Accept: 'application/json',
      },
    });
  } catch (err) {
    // Network-level failure (server down, DNS failure, CORS block, etc.)
    throw new ConnectionError(path, err);
  }

  // Vite proxy returns 500 (or 502/503/504) when backend is unreachable
  if (response.status >= 500) {
    throw new ConnectionError(path);
  }

  if (!response.ok) {
    throw new Error(`Admin API ${path} failed with ${response.status}`);
  }

  return response.json() as Promise<T>;
}

export function formatUptime(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const remainingSeconds = seconds % 60;

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }

  if (minutes > 0) {
    return `${minutes}m ${remainingSeconds}s`;
  }

  return `${remainingSeconds}s`;
}

export function buildDashboardStats(
  status: AdminStatusResponse,
  players: PlayersResponse,
  spaces: SpacesResponse,
  content: ContentSummaryResponse,
): DashboardStat[] {
  return [
    {
      label: 'Uptime',
      value: formatUptime(status.uptime_seconds),
      detail: 'Derived directly from the running orchestrator uptime counter.',
    },
    {
      label: 'Players Online',
      value: String(players.summary.online_count),
      detail: players.available
        ? 'Live roster reported by the backend.'
        : players.reason ?? 'Live roster source is not available yet.',
    },
    {
      label: 'Known Spaces',
      value: String(spaces.summary.total_spaces),
      detail: spaces.available
        ? `${spaces.summary.scripted_spaces} spaces currently have scripts.`
        : spaces.reason ?? 'Space catalog is unavailable.',
    },
    {
      label: 'Mission Records',
      value: String(content.summary.mission_count),
      detail: content.available
        ? `${content.summary.scripted_mission_count} missions currently reference a scripted space.`
        : content.reason ?? 'Mission summary is unavailable.',
    },
  ];
}

export function buildServiceHealth(status: AdminStatusResponse): ServiceHealthCard[] {
  return [
    {
      name: 'Authentication',
      status: status.services.auth ? 'Running' : 'Stopped',
      detail: 'Account logins and shard selection.',
      variant: status.services.auth ? 'success' : 'destructive',
    },
    {
      name: 'Base Service',
      status: status.services.base ? 'Running' : 'Stopped',
      detail: 'Session routing and player presence.',
      variant: status.services.base ? 'success' : 'destructive',
    },
    {
      name: 'Cell Service',
      status: status.services.cell ? 'Running' : 'Stopped',
      detail: 'World simulation and space instances.',
      variant: status.services.cell ? 'success' : 'destructive',
    },
    {
      name: 'Database',
      status: status.services.database ? 'Reachable' : 'Unavailable',
      detail: 'PostgreSQL-backed resources and editor persistence.',
      variant: status.services.database ? 'success' : 'outline',
    },
  ];
}

export function buildDashboardActivity(
  status: AdminStatusResponse,
  spaces: SpacesResponse,
  content: ContentSummaryResponse,
): DashboardActivity[] {
  const now = new Date().toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
  });

  return [
    {
      time: now,
      title: status.services.database ? 'Database link healthy' : 'Database link unavailable',
      detail: status.services.database
        ? 'Resource-backed pages are reading from PostgreSQL.'
        : 'The shell is running, but database-backed surfaces are in degraded mode.',
    },
    {
      time: now,
      title: `${spaces.summary.total_spaces} spaces loaded`,
      detail: spaces.available
        ? `${spaces.summary.mission_links} mission-to-space links were discovered from the seed data.`
        : spaces.reason ?? 'Space catalog could not be loaded.',
    },
    {
      time: now,
      title: `${content.summary.mission_count} missions indexed`,
      detail: content.available
        ? `${content.summary.story_mission_count} story missions are enabled for operator browsing.`
        : content.reason ?? 'Mission summary could not be loaded.',
    },
  ];
}

export function buildConfigSections(config: AdminConfigResponse) {
  return [
    {
      title: 'Network',
      items: [
        { label: 'Auth Host', value: config.config.auth_host },
        { label: 'Auth Port', value: String(config.config.auth_port) },
        { label: 'Base Host', value: config.config.base_host },
        { label: 'Base Port', value: String(config.config.base_port) },
        { label: 'Cell Host', value: config.config.cell_host },
        { label: 'Cell Port', value: String(config.config.cell_port) },
        { label: 'Admin API Port', value: String(config.config.admin_port) },
      ],
    },
    {
      title: 'Runtime',
      items: [
        {
          label: 'Developer Mode',
          value: config.config.developer_mode ? 'true' : 'false',
        },
      ],
    },
  ];
}

export async function fetchAdminConfig() {
  return fetchAdminJson<AdminConfigResponse>('/api/config');
}

export async function fetchAdminStatus() {
  return fetchAdminJson<AdminStatusResponse>('/api/config/status');
}

export async function fetchPlayers() {
  return fetchAdminJson<PlayersResponse>('/api/players');
}

export async function fetchSpaces() {
  return fetchAdminJson<SpacesResponse>('/api/spaces');
}

export async function fetchContentSummary() {
  return fetchAdminJson<ContentSummaryResponse>('/api/content/summary');
}

export async function fetchContentEditorPickers() {
  return fetchAdminJson<ContentEditorPickersResponse>('/api/content/editor-pickers');
}
