import { Route, Router } from '@solidjs/router';
import AppShell from './components/AppShell';
import ChainEditor from './pages/ChainEditor';
import Config from './pages/Config';
import ContentEditor from './pages/ContentEditor';
import Dashboard from './pages/Dashboard';
import Logs from './pages/Logs';
import Players from './pages/Players';
import SpaceViewer from './pages/SpaceViewer';

export default function App() {
  return (
    <Router root={AppShell}>
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
