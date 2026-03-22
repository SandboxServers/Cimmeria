import { NavLink, Outlet, useLocation } from 'react-router-dom';
import { useMemo, useState, type ComponentType } from 'react';
import type { LucideProps } from 'lucide-react';
import {
  Boxes,
  Compass,
  LayoutDashboard,
  Menu,
  ScrollText,
  Settings2,
  ShieldCheck,
  Sparkles,
  UsersRound,
} from 'lucide-react';
import ServerControls from './ServerControls';
import { Badge } from './ui/badge';
import { Button } from './ui/button';
import { cn } from '../lib/utils';

type NavItem = {
  path: string;
  label: string;
  hint: string;
  icon: ComponentType<LucideProps>;
};

const navItems: NavItem[] = [
  { path: '/', label: 'Dashboard', hint: 'Fleet telemetry', icon: LayoutDashboard },
  { path: '/players', label: 'Players', hint: 'Live sessions', icon: UsersRound },
  { path: '/content-editor', label: 'Content Editor', hint: 'Game data', icon: Boxes },
  { path: '/space-viewer', label: 'Space Viewer', hint: 'World map', icon: Compass },
  { path: '/logs', label: 'Logs', hint: 'Event stream', icon: ScrollText },
  { path: '/config', label: 'Config', hint: 'Runtime tuning', icon: Settings2 },
];

export default function AppShell() {
  const location = useLocation();
  const [mobileOpen, setMobileOpen] = useState(false);
  const showPageChrome = true;

  const activeItem = useMemo(
    () => navItems.find((item) => item.path === location.pathname) ?? navItems[0],
    [location.pathname],
  );

  const Sidebar = () => (
    <aside className="subtle-scrollbar flex h-full w-full flex-col overflow-y-auto border border-[rgba(77,70,53,0.15)] bg-surface-lowest p-5">
      <div className="obsidian-panel border border-[rgba(77,70,53,0.15)] p-5">
        <div className="mb-4 flex items-center justify-between">
          <Badge>Phase 4</Badge>
          <Sparkles className="size-4 text-primary" />
        </div>
        <h2 className="font-headline text-xl tracking-tighter uppercase text-primary">
          COMMAND_STRATA
        </h2>
        <p className="mt-2 text-sm leading-6 text-on-surface-variant">
          Operations-grade admin tooling for the emulator rewrite. Tailwind-driven UI, ready for API wiring.
        </p>
      </div>

      <div className="mt-6 flex items-center justify-between px-2">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.28em] text-on-surface-variant">
            Control Surfaces
          </p>
          <p className="mt-1 text-sm text-on-surface">Phase 4 dashboard stack</p>
        </div>
        <Badge variant="success">Online</Badge>
      </div>

      <nav className="mt-4 space-y-2">
        {navItems.map((item) => {
          const Icon = item.icon;
          return (
            <NavLink
              key={item.path}
              className={({ isActive }) =>
                cn(
                  'group flex items-center justify-between border border-transparent px-3 py-3 transition-all duration-200 hover:bg-surface-highest hover:translate-x-1',
                  isActive && 'bg-secondary-container/30 text-on-surface shadow-[inset_4px_0_0_0_#F2CA50]',
                )
              }
              to={item.path}
              end={item.path === '/'}
              onClick={() => setMobileOpen(false)}
            >
              <div className="flex items-center gap-3">
                <div className="border border-[rgba(77,70,53,0.15)] bg-surface-high p-2 text-on-surface-variant transition-colors">
                  <Icon className="size-4" />
                </div>
                <div>
                  <p className="text-sm font-medium text-on-surface">{item.label}</p>
                  <p className="text-xs text-on-surface-variant">{item.hint}</p>
                </div>
              </div>
            </NavLink>
          );
        })}
      </nav>

      <div className="mt-auto border border-[rgba(77,70,53,0.15)] bg-surface-high p-4">
        <div className="flex items-center gap-3">
          <div className="bg-accent/15 p-2 text-accent">
            <ShieldCheck className="size-4" />
          </div>
          <div>
            <p className="text-sm font-medium text-on-surface">Local operator mode</p>
            <p className="text-xs text-on-surface-variant">JWT and Tauri IPC hooks can slot in here.</p>
          </div>
        </div>
      </div>
    </aside>
  );

  return (
    <div className="relative min-h-screen">
      <div className="flex min-h-screen w-full gap-4 p-4 lg:gap-6 lg:p-6">
        <div className="hidden w-[308px] shrink-0 lg:block">
          <Sidebar />
        </div>

        {mobileOpen && (
          <>
            <button
              aria-label="Close navigation overlay"
              className="fixed inset-0 z-40 bg-black/55 backdrop-blur-sm lg:hidden"
              onClick={() => setMobileOpen(false)}
              type="button"
            />
            <div className="fixed inset-y-4 left-4 z-50 w-[min(82vw,320px)] lg:hidden">
              <Sidebar />
            </div>
          </>
        )}

        <main className="flex min-h-[calc(100vh-2rem)] min-w-0 flex-1 flex-col gap-6 border border-[rgba(77,70,53,0.15)] bg-surface p-4 shadow-[0_30px_120px_rgba(0,0,0,0.24)] sm:p-5 lg:p-6">
          {showPageChrome && (
            <header className="flex flex-col gap-4 border border-[rgba(77,70,53,0.15)] bg-surface-high px-4 py-4 sm:px-5 lg:flex-row lg:items-center lg:justify-between">
              <div className="flex items-start gap-3">
                <Button
                  className="lg:hidden"
                  onClick={() => setMobileOpen(true)}
                  size="icon"
                  variant="outline"
                >
                  <Menu className="size-4" />
                </Button>
                <div className="space-y-2">
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline">Admin Dashboard</Badge>
                    <Badge variant="success">Realtime-ready</Badge>
                  </div>
                  <div>
                    <h1 className="text-lg font-semibold tracking-[-0.03em] text-on-surface sm:text-xl">
                      {activeItem.label}
                    </h1>
                    <p className="text-sm text-on-surface-variant">
                      {activeItem.hint} with a Tailwind and shadcn-style operator interface.
                    </p>
                  </div>
                </div>
              </div>

              <ServerControls />
            </header>
          )}

          <div className="min-w-0 flex-1">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
