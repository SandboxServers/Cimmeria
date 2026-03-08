const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

// --- State machine ---
const states = ['not-installed', 'downloading', 'extracting', 'ready', 'updating', 'error'];
let currentState = null;
let previousState = null;

function showState(name) {
  previousState = currentState;
  currentState = name;
  for (const s of states) {
    const el = document.getElementById('state-' + s);
    el.hidden = s !== name;
  }
}

// --- Elements ---
const installPathInput = document.getElementById('install-path');
const serverAddressInput = document.getElementById('server-address');
const downloadBar = document.getElementById('download-bar');
const downloadPercent = document.getElementById('download-percent');
const downloadSpeed = document.getElementById('download-speed');
const downloadEta = document.getElementById('download-eta');
const extractBar = document.getElementById('extract-bar');
const extractPhase = document.getElementById('extract-phase');
const extractFile = document.getElementById('extract-file');
const updateBar = document.getElementById('update-bar');
const updateCount = document.getElementById('update-count');
const updateFile = document.getElementById('update-file');
const errorMessage = document.getElementById('error-message');

// --- Helpers ---
function formatBytes(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + ' MB';
  return (bytes / 1073741824).toFixed(1) + ' GB';
}

function formatSpeed(bps) {
  return formatBytes(bps) + '/s';
}

function formatEta(secs) {
  if (secs < 60) return secs + 's';
  if (secs < 3600) return Math.floor(secs / 60) + 'm ' + (secs % 60) + 's';
  return Math.floor(secs / 3600) + 'h ' + Math.floor((secs % 3600) / 60) + 'm';
}

function showError(msg) {
  errorMessage.textContent = msg;
  showState('error');
}

// --- Event listeners ---
listen('download-progress', (event) => {
  const p = event.payload;
  downloadBar.style.width = p.percent.toFixed(1) + '%';
  downloadPercent.textContent = p.percent.toFixed(1) + '%';
  downloadSpeed.textContent = formatSpeed(p.speed_bps);
  downloadEta.textContent = formatEta(p.eta_secs);
});

listen('extract-progress', (event) => {
  const p = event.payload;
  showState('extracting');
  const pct = p.total > 0 ? (p.current / p.total * 100) : 0;
  extractBar.style.width = pct.toFixed(1) + '%';
  extractPhase.textContent = p.phase === 'rar' ? 'Extracting archive...' : 'CAB ' + p.current + '/' + p.total;
  extractFile.textContent = p.filename;
});

listen('install-complete', () => {
  showState('ready');
});

listen('install-error', (event) => {
  showError(event.payload.message);
});

listen('update-progress', (event) => {
  const p = event.payload;
  const pct = p.total > 0 ? (p.current / p.total * 100) : 0;
  updateBar.style.width = pct.toFixed(1) + '%';
  updateCount.textContent = p.current + '/' + p.total;
  updateFile.textContent = p.filename;
});

listen('update-complete', () => {
  showState('ready');
});

// --- Button handlers ---
document.getElementById('btn-browse').addEventListener('click', async () => {
  const selected = await open({ directory: true, title: 'Select install location' });
  if (selected) {
    installPathInput.value = selected;
  }
});

document.getElementById('btn-install').addEventListener('click', async () => {
  const path = installPathInput.value;
  if (!path) {
    showError('Please select an install location.');
    return;
  }
  showState('downloading');
  try {
    const config = await invoke('cmd_load_config');
    await invoke('cmd_download_and_install', {
      installPath: path,
      serverAddress: config.server_address,
    });
  } catch (e) {
    showError(typeof e === 'string' ? e : e.message || 'Install failed');
  }
});

document.getElementById('btn-cancel').addEventListener('click', async () => {
  await invoke('cmd_cancel_install');
  showState('not-installed');
});

document.getElementById('btn-play').addEventListener('click', async () => {
  try {
    await invoke('cmd_launch_game');
  } catch (e) {
    showError(typeof e === 'string' ? e : e.message || 'Launch failed');
  }
});

document.getElementById('btn-check-updates').addEventListener('click', async () => {
  try {
    const result = await invoke('cmd_check_for_updates');
    if (result.updates_available) {
      showState('updating');
      await invoke('cmd_apply_updates');
    }
  } catch (e) {
    showError(typeof e === 'string' ? e : e.message || 'Update check failed');
  }
});

document.getElementById('btn-retry').addEventListener('click', () => {
  showState(previousState || 'not-installed');
});

// Server address: save on blur
serverAddressInput.addEventListener('blur', async () => {
  const address = serverAddressInput.value.trim();
  if (!address) return;
  try {
    const config = await invoke('cmd_load_config');
    config.server_address = address;
    await invoke('cmd_save_config', { config });
    await invoke('cmd_patch_server_address', { address });
  } catch (e) {
    showError(typeof e === 'string' ? e : e.message || 'Failed to update server address');
  }
});

// --- Initialization ---
async function init() {
  try {
    const config = await invoke('cmd_load_config');
    serverAddressInput.value = config.server_address;
    installPathInput.value = config.install_path || await invoke('cmd_get_default_install_path');

    const state = await invoke('cmd_check_installation');
    if (state === 'Installed') {
      showState('ready');
    } else {
      showState('not-installed');
    }
  } catch (e) {
    showError('Failed to initialize: ' + (typeof e === 'string' ? e : e.message));
  }
}

init();
