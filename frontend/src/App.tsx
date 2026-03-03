import { Router, Route, A } from '@solidjs/router';
import Dashboard from './pages/Dashboard';
import Players from './pages/Players';
import ContentEditor from './pages/ContentEditor';
import ChainEditor from './pages/ChainEditor';
import SpaceViewer from './pages/SpaceViewer';
import Logs from './pages/Logs';
import Config from './pages/Config';

const navItems = [
  { path: '/', label: 'Dashboard' },
  { path: '/players', label: 'Players' },
  { path: '/content-editor', label: 'Content Editor' },
  { path: '/chain-editor', label: 'Chain Editor' },
  { path: '/space-viewer', label: 'Space Viewer' },
  { path: '/logs', label: 'Logs' },
  { path: '/config', label: 'Config' },
];

function Layout(props: { children?: any }) {
  return (
    <div style={{ display: 'flex', height: '100vh', "font-family": 'system-ui, sans-serif' }}>
      <nav style={{
        width: '220px',
        background: '#1a1a2e',
        color: '#e0e0e0',
        display: 'flex',
        "flex-direction": 'column',
        padding: '0',
        "flex-shrink": '0',
      }}>
        <div style={{
          padding: '20px 16px',
          "font-size": '18px',
          "font-weight": 'bold',
          "border-bottom": '1px solid #2a2a4a',
          color: '#ffffff',
        }}>
          Cimmeria Admin
        </div>
        <ul style={{ "list-style": 'none', padding: '8px 0', margin: '0' }}>
          {navItems.map((item) => (
            <li>
              <A
                href={item.path}
                style={{
                  display: 'block',
                  padding: '10px 16px',
                  color: '#c0c0c0',
                  "text-decoration": 'none',
                  "font-size": '14px',
                }}
                activeClass="active-nav"
              >
                {item.label}
              </A>
            </li>
          ))}
        </ul>
      </nav>
      <main style={{ flex: '1', padding: '24px', overflow: 'auto', background: '#f5f5f5' }}>
        {props.children}
      </main>
    </div>
  );
}

export default function App() {
  return (
    <Router root={Layout}>
      <Route path="/" component={Dashboard} />
      <Route path="/players" component={Players} />
      <Route path="/content-editor" component={ContentEditor} />
      <Route path="/chain-editor" component={ChainEditor} />
      <Route path="/space-viewer" component={SpaceViewer} />
      <Route path="/logs" component={Logs} />
      <Route path="/config" component={Config} />
    </Router>
  );
}
