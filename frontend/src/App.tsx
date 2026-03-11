import { BrowserRouter, Route, Routes } from 'react-router-dom';
import AppShell from './components/AppShell';
import Config from './pages/Config';
import ContentEditor from './pages/ContentEditor';
import Dashboard from './pages/Dashboard';
import Logs from './pages/Logs';
import Players from './pages/Players';
import SpaceViewer from './pages/SpaceViewer';

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/" element={<Dashboard />} />
          <Route path="/players" element={<Players />} />
          <Route path="/content-editor" element={<ContentEditor />} />
          <Route path="/space-viewer" element={<SpaceViewer />} />
          <Route path="/logs" element={<Logs />} />
          <Route path="/config" element={<Config />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
