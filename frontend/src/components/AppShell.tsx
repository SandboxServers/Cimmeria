import { NavLink, Outlet, useLocation } from 'react-router-dom';
import { useMemo, useState, type ComponentType } from 'react';
import type { LucideProps } from 'lucide-react';
import {
  Boxes,
  Cable,
  ChevronRight,
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
  { path: '/chain-editor', label: 'Chain Editor', hint: 'Mission logic', icon: Cable },
  { path: '/space-viewer', label: 'Space Viewer', hint: 'World map', icon: Compass },
  { path: '/logs', label: 'Logs', hint: 'Event stream', icon: ScrollText },
  { path: '/config', label: 'Config', hint: 'Runtime tuning', icon: Settings2 },
];

export default function AppShell() {
  const location = useLocation();
  const [mobileOpen, setMobileOpen] = useState(false);
  const showPageChrome = location.pathname !== '/chain-editor';

  const activeItem = useMemo(
    () => navItems.find((item) => item.path === location.pathname) ?? navItems[0],
    [location.pathname],
  );

  const Sidebar = () => (
    <aside className="glass-panel subtle-scrollbar flex h-full w-full flex-col overflow-y-auto rounded-[30px] border border-white/8 bg-slate-950/82 p-5">
      <div className="rounded-[28px] border border-white/6 bg-gradient-to-br from-primary/16 via-transparent to-accent/12 p-5">
        <div className="mb-4 flex items-center justify-between">
          <Badge>Phase 4</Badge>
          <Sparkles className="size-4 text-primary" />
        </div>
        <h2 className="text-xl font-semibold tracking-[-0.04em] text-foreground">
          Cimmeria Command
        </h2>
        <p className="mt-2 text-sm leading-6 text-muted-foreground">
          Operations-grade admin tooling for the emulator rewrite. Tailwind-driven UI, ready for API wiring.
        </p>
      </div>

      <div className="mt-6 flex items-center justify-between px-2">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.28em] text-muted-foreground">
            Control Surfaces
          </p>
          <p className="mt-1 text-sm text-foreground">Phase 4 dashboard stack</p>
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
                  'group flex items-center justify-between rounded-[24px] border border-transparent px-3 py-3 transition-all hover:border-white/10 hover:bg-white/[0.045]',
                  isActive && 'border-primary/40 bg-primary/14 text-foreground shadow-[0_14px_40px_rgba(245,170,49,0.14)]',
                )
              }
              to={item.path}
              end={item.path === '/'}
              onClick={() => setMobileOpen(false)}
            >
              <div className="flex items-center gap-3">
                <div className="rounded-2xl border border-white/8 bg-white/6 p-2 text-muted-foreground transition-colors">
                  <Icon className="size-4" />
                </div>
                <div>
                  <p className="text-sm font-medium text-foreground">{item.label}</p>
                  <p className="text-xs text-muted-foreground">{item.hint}</p>
                </div>
              </div>
              <ChevronRight className="size-4 text-muted-foreground transition-transform group-hover:translate-x-0.5" />
            </NavLink>
          );
        })}
      </nav>

      <div className="mt-auto rounded-[28px] border border-white/6 bg-white/5 p-4">
        <div className="flex items-center gap-3">
          <div className="rounded-full bg-accent/15 p-2 text-accent">
            <ShieldCheck className="size-4" />
          </div>
          <div>
            <p className="text-sm font-medium text-foreground">Local operator mode</p>
            <p className="text-xs text-muted-foreground">JWT and Tauri IPC hooks can slot in here.</p>
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

        <main className="flex min-h-[calc(100vh-2rem)] min-w-0 flex-1 flex-col gap-6 rounded-[34px] border border-white/6 bg-slate-950/32 p-4 shadow-[0_30px_120px_rgba(0,0,0,0.24)] sm:p-5 lg:p-6">
          {showPageChrome && (
            <header className="glass-panel flex flex-col gap-4 rounded-[28px] border border-white/8 px-4 py-4 sm:px-5 lg:flex-row lg:items-center lg:justify-between">
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
                    <h1 className="text-lg font-semibold tracking-[-0.03em] text-foreground sm:text-xl">
                      {activeItem.label}
                    </h1>
                    <p className="text-sm text-muted-foreground">
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
