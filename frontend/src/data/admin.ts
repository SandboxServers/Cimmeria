export const dashboardStats = [
  {
    label: 'Uptime',
    value: '18h 42m',
    delta: '+2.4%',
    detail: 'No service restarts since the last schema sync.',
  },
  {
    label: 'Players Online',
    value: '37',
    delta: '+6',
    detail: 'Peak concurrency is trending above the weekly median.',
  },
  {
    label: 'Active Entities',
    value: '1,482',
    delta: '+14%',
    detail: 'Includes AI, lootables, interactables, and mission actors.',
  },
  {
    label: 'Live Errors',
    value: '3',
    delta: '-5',
    detail: 'Only one incident remains above the warning threshold.',
  },
];

export const serviceHealth = [
  { name: 'Authentication', status: 'Healthy', latency: '31ms', load: 68 },
  { name: 'Base Service', status: 'Stable', latency: '14ms', load: 54 },
  { name: 'Cell Service', status: 'Scaling', latency: '27ms', load: 79 },
  { name: 'Admin API', status: 'Healthy', latency: '22ms', load: 46 },
];

export const liveActivity = [
  {
    time: '09:04',
    title: 'Harset content publish',
    detail: 'Mission rewards sync completed with zero schema drift.',
  },
  {
    time: '08:47',
    title: 'Entity stream recovered',
    detail: 'Reconnected 2 websocket subscribers after a local restart.',
  },
  {
    time: '08:31',
    title: 'Spawn budget alert',
    detail: 'Castle_CellBlock hit 91% of configured mob density.',
  },
];

export const releaseChecklist = [
  { label: 'REST routes stubbed', complete: true },
  { label: 'WebSocket feed contracts', complete: true },
  { label: 'Content editor publish flow', complete: true },
  { label: 'Remote auth polish', complete: false },
];

export const players = [
  {
    name: 'Teyla Voss',
    archetype: 'Recon Specialist',
    level: 31,
    zone: 'Harset / Command Center',
    ping: 42,
    status: 'In mission',
    session: '02h 14m',
  },
  {
    name: 'Kael Mercer',
    archetype: 'Heavy Assault',
    level: 28,
    zone: 'Castle / Cellblock',
    ping: 61,
    status: 'Combat',
    session: '00h 56m',
  },
  {
    name: 'Nyra Sol',
    archetype: 'Field Medic',
    level: 24,
    zone: 'SGC_W1 / Security Office',
    ping: 38,
    status: 'Idle',
    session: '03h 03m',
  },
  {
    name: 'Dax Rowan',
    archetype: 'Engineer',
    level: 19,
    zone: 'Omega Site / Gate Room',
    ping: 49,
    status: 'Crafting',
    session: '01h 22m',
  },
];

export const contentDomains = [
  {
    name: 'Missions',
    records: 184,
    owner: 'Narrative Systems',
    summary: 'Steps, objectives, reward groups, minigame calls.',
  },
  {
    name: 'Items',
    records: 618,
    owner: 'Loot Economy',
    summary: 'Item lists, pricing, blueprint bindings, inventory payloads.',
  },
  {
    name: 'Abilities',
    records: 322,
    owner: 'Combat Design',
    summary: 'Ability sets, cooldown categories, trainer mappings.',
  },
  {
    name: 'Dialogs',
    records: 204,
    owner: 'Narrative Systems',
    summary: 'Dialog screens, buttons, speaker routing, special words.',
  },
];

export const publishQueue = [
  { target: 'Harset/GivingTheWallsEars', author: 'design/akline', state: 'Ready' },
  { target: 'Castle_CellBlock/Aftermath', author: 'design/jng', state: 'Review' },
  { target: 'Weapons/ability_sets', author: 'combat/rye', state: 'Queued' },
];

export const chainNodes = [
  {
    stage: 'Trigger',
    title: 'Mission Accepted',
    detail: 'On accept, seed prisoner objective and unlock hallway feed.',
  },
  {
    stage: 'Condition',
    title: 'Inventory Contains Keycard',
    detail: 'Gate progression behind lootable recovery or GM override.',
  },
  {
    stage: 'Action',
    title: 'Broadcast Security Response',
    detail: 'Spawn guards, push dialog cue, and mark alarm state.',
  },
  {
    stage: 'Result',
    title: 'Advance to EscapeTheCellblock',
    detail: 'Persist quest state and reopen transport interaction.',
  },
];

export const logEntries = [
  {
    level: 'INFO',
    timestamp: '09:12:03',
    source: 'admin-api',
    message: 'GET /api/dashboard 200 7ms',
  },
  {
    level: 'WARN',
    timestamp: '09:11:41',
    source: 'cell-service',
    message: 'spawn budget exceeded in Castle_CellBlock by 6 entities',
  },
  {
    level: 'ERROR',
    timestamp: '09:09:14',
    source: 'content-engine',
    message: 'chain validation failed for mission reward group 112',
  },
  {
    level: 'DEBUG',
    timestamp: '09:07:28',
    source: 'entity-stream',
    message: 'player 2017 position diff compressed to 22 bytes',
  },
];

export const configSections = [
  {
    title: 'Network',
    items: [
      { label: 'Auth Port', value: '13001' },
      { label: 'Base Port', value: '32832' },
      { label: 'Cell Port', value: '50000' },
      { label: 'Admin API Port', value: '8443' },
    ],
  },
  {
    title: 'Runtime',
    items: [
      { label: 'Developer Mode', value: 'false' },
      { label: 'Trace Sampling', value: '0.25' },
      { label: 'Entity Snapshot Window', value: '120ms' },
      { label: 'Hot Reload Safety', value: 'strict' },
    ],
  },
  {
    title: 'Storage',
    items: [
      { label: 'PostgreSQL', value: 'localhost:5433/sgw' },
      { label: 'Cooked Data Cache', value: 'data/cache/' },
      { label: 'Mission Scripts', value: 'data/scripts/missions/' },
      { label: 'Nav Meshes', value: 'data/spaces/' },
    ],
  },
];

export const spaceLayers = [
  { label: 'Nav Mesh Chunks', enabled: true, count: 124 },
  { label: 'Spawn Regions', enabled: true, count: 37 },
  { label: 'Live Players', enabled: true, count: 4 },
  { label: 'Ghost Entities', enabled: false, count: 82 },
];
