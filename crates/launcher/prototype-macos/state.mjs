// THROWAWAY PROTOTYPE. In-memory simulation; never touches a game or Wine.
export const initial = () => ({phase:'welcome', step:0, telemetry:false, details:false, tab:'play', settings:false, confirmUninstall:false, notice:''});
export const steps = ['Checking your Mac', 'Preparing game compatibility', 'Downloading Stargate Worlds', 'Installing verified patches', 'Checking the game'];
export function transition(s, action) {
  if (action === 'settings') return {...s, settings:!s.settings, confirmUninstall:false, notice:''};
  if (action === 'open-folder') return {...s, notice:'Demo only: Finder would open your local game folder.'};
  if (action === 'repair-game' && ['ready','repair'].includes(s.phase)) return {...s, phase:'installing', step:3, settings:false, tab:'play', notice:''};
  if (action === 'uninstall' && ['ready','repair'].includes(s.phase)) return {...s, confirmUninstall:true};
  if (action === 'cancel-uninstall') return {...s, confirmUninstall:false};
  if (action === 'confirm-uninstall' && s.confirmUninstall && ['ready','repair'].includes(s.phase)) return {...s, phase:'welcome', step:0, settings:false, confirmUninstall:false, tab:'play', notice:''};
  if (action === 'tab-play') return {...s, tab:'play'};
  if (action === 'tab-notes') return {...s, tab:'notes'};
  if (action === 'reset') return initial();
  if (action === 'telemetry') return {...s, telemetry:!s.telemetry};
  if (action === 'details') return {...s, details:!s.details};
  if (action === 'install' && s.phase === 'welcome') return {...s, phase:'installing', step:0};
  if (action === 'advance' && s.phase === 'installing') return s.step < steps.length-1 ? {...s, step:s.step+1} : {...s,phase:'ready'};
  if (action === 'fail' && ['installing','ready'].includes(s.phase)) return {...s,phase:'repair'};
  if (action === 'repair' && s.phase === 'repair') return {...s,phase:'installing',step:1};
  if (action === 'play' && s.phase === 'ready') return {...s,phase:'playing'};
  if (action === 'return' && s.phase === 'playing') return {...s,phase:'ready'};
  return s;
}
