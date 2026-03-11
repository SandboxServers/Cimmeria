import { describe, expect, it } from 'vitest';
import {
  buildConfigSections,
  buildDashboardStats,
  buildServiceHealth,
  formatUptime,
  type AdminConfigResponse,
  type AdminStatusResponse,
  type ContentSummaryResponse,
  type PlayersResponse,
  type SpacesResponse,
} from './admin-api';

const status: AdminStatusResponse = {
  status: 'ok',
  uptime_seconds: 7265,
  services: {
    auth: true,
    base: true,
    cell: false,
    database: true,
  },
};

const players: PlayersResponse = {
  status: 'ok',
  available: false,
  reason: 'Live player roster is not implemented yet.',
  players: [],
  summary: {
    online_count: 0,
    ready: false,
  },
  services: status.services,
};

const spaces: SpacesResponse = {
  status: 'ok',
  available: true,
  reason: null,
  spaces: [],
  summary: {
    total_spaces: 12,
    scripted_spaces: 4,
    mission_links: 18,
  },
};

const content: ContentSummaryResponse = {
  status: 'ok',
  available: true,
  reason: null,
  summary: {
    world_count: 12,
    scripted_world_count: 4,
    mission_count: 88,
    story_mission_count: 31,
    hidden_mission_count: 6,
    scripted_mission_count: 22,
  },
  top_space_mission_counts: [],
};

const config: AdminConfigResponse = {
  status: 'ok',
  config: {
    auth_host: '127.0.0.1',
    auth_port: 13001,
    base_host: '127.0.0.1',
    base_port: 32832,
    cell_host: '127.0.0.1',
    cell_port: 50000,
    admin_port: 8443,
    developer_mode: false,
  },
};

describe('admin-api helpers', () => {
  it('formats uptime into a readable operator string', () => {
    expect(formatUptime(12)).toBe('12s');
    expect(formatUptime(95)).toBe('1m 35s');
    expect(formatUptime(7265)).toBe('2h 1m');
  });

  it('builds dashboard stats from live backend summaries', () => {
    expect(buildDashboardStats(status, players, spaces, content)).toEqual([
      {
        label: 'Uptime',
        value: '2h 1m',
        detail: 'Derived directly from the running orchestrator uptime counter.',
      },
      {
        label: 'Players Online',
        value: '0',
        detail: 'Live player roster is not implemented yet.',
      },
      {
        label: 'Known Spaces',
        value: '12',
        detail: '4 spaces currently have scripts.',
      },
      {
        label: 'Mission Records',
        value: '88',
        detail: '22 missions currently reference a scripted space.',
      },
    ]);
  });

  it('maps service booleans into frontend health cards', () => {
    expect(buildServiceHealth(status)).toEqual([
      {
        name: 'Authentication',
        status: 'Running',
        detail: 'Account logins and shard selection.',
        variant: 'success',
      },
      {
        name: 'Base Service',
        status: 'Running',
        detail: 'Session routing and player presence.',
        variant: 'success',
      },
      {
        name: 'Cell Service',
        status: 'Stopped',
        detail: 'World simulation and space instances.',
        variant: 'destructive',
      },
      {
        name: 'Database',
        status: 'Reachable',
        detail: 'PostgreSQL-backed resources and editor persistence.',
        variant: 'success',
      },
    ]);
  });

  it('builds readonly config sections from the live config payload', () => {
    expect(buildConfigSections(config)).toEqual([
      {
        title: 'Network',
        items: [
          { label: 'Auth Host', value: '127.0.0.1' },
          { label: 'Auth Port', value: '13001' },
          { label: 'Base Host', value: '127.0.0.1' },
          { label: 'Base Port', value: '32832' },
          { label: 'Cell Host', value: '127.0.0.1' },
          { label: 'Cell Port', value: '50000' },
          { label: 'Admin API Port', value: '8443' },
        ],
      },
      {
        title: 'Runtime',
        items: [{ label: 'Developer Mode', value: 'false' }],
      },
    ]);
  });
});
