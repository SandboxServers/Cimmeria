import { A, useLocation } from '@solidjs/router';
import { createMemo, createSignal, For, Show, type JSX } from 'solid-js';
import {
  Activity,
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
} from 'lucide-solid';
import { Badge } from './ui/badge';
import { Button } from './ui/button';

type NavItem = {
  path: string;
  label: string;
  hint: string;
  icon: (props: JSX.SvgSVGAttributes<SVGSVGElement>) => JSX.Element;
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

export default function AppShell(props: { children?: JSX.Element }) {
  const location = useLocation();
  const [mobileOpen, setMobileOpen] = createSignal(false);

  const activeItem = createMemo(
    () => navItems.find((item) => item.path === location.pathname) ?? navItems[0],
  );

  const Sidebar = () => (
    <aside class="glass-panel subtle-scrollbar flex h-full w-full flex-col overflow-y-auto rounded-[30px] border border-white/8 bg-slate-950/82 p-5">
      <div class="rounded-[28px] border border-white/6 bg-gradient-to-br from-primary/16 via-transparent to-accent/12 p-5">
        <div class="mb-4 flex items-center justify-between">
          <Badge>Phase 4</Badge>
          <Sparkles class="size-4 text-primary" />
        </div>
        <h2 class="text-xl font-semibold tracking-[-0.04em] text-foreground">
          Cimmeria Command
        </h2>
        <p class="mt-2 text-sm leading-6 text-muted-foreground">
          Operations-grade admin tooling for the emulator rewrite. Tailwind-driven UI, ready for API wiring.
        </p>
      </div>

      <div class="mt-6 flex items-center justify-between px-2">
        <div>
          <p class="text-xs font-semibold uppercase tracking-[0.28em] text-muted-foreground">
            Control Surfaces
          </p>
          <p class="mt-1 text-sm text-foreground">Phase 4 dashboard stack</p>
        </div>
        <Badge variant="success">Online</Badge>
      </div>

      <nav class="mt-4 space-y-2">
        <For each={navItems}>
          {(item) => {
            const Icon = item.icon;
            return (
              <A
                activeClass="border-primary/40 bg-primary/14 text-foreground shadow-[0_14px_40px_rgba(245,170,49,0.14)]"
                class="group flex items-center justify-between rounded-[24px] border border-transparent px-3 py-3 transition-all hover:border-white/10 hover:bg-white/[0.045]"
                href={item.path}
                onClick={() => setMobileOpen(false)}
              >
                <div class="flex items-center gap-3">
                  <div class="rounded-2xl border border-white/8 bg-white/6 p-2 text-muted-foreground transition-colors">
                    <Icon class="size-4" />
                  </div>
                  <div>
                    <p class="text-sm font-medium text-foreground">{item.label}</p>
                    <p class="text-xs text-muted-foreground">{item.hint}</p>
                  </div>
                </div>
                <ChevronRight class="size-4 text-muted-foreground transition-transform group-hover:translate-x-0.5" />
              </A>
            );
          }}
        </For>
      </nav>

      <div class="mt-auto rounded-[28px] border border-white/6 bg-white/5 p-4">
        <div class="flex items-center gap-3">
          <div class="rounded-full bg-accent/15 p-2 text-accent">
            <ShieldCheck class="size-4" />
          </div>
          <div>
            <p class="text-sm font-medium text-foreground">Local operator mode</p>
            <p class="text-xs text-muted-foreground">JWT and Tauri IPC hooks can slot in here.</p>
          </div>
        </div>
      </div>
    </aside>
  );

  return (
    <div class="relative min-h-screen">
      <div class="flex min-h-screen w-full gap-4 p-4 lg:gap-6 lg:p-6">
        <div class="hidden w-[308px] shrink-0 lg:block">
          <Sidebar />
        </div>

        <Show when={mobileOpen()}>
          <button
            aria-label="Close navigation overlay"
            class="fixed inset-0 z-40 bg-black/55 backdrop-blur-sm lg:hidden"
            onClick={() => setMobileOpen(false)}
            type="button"
          />
          <div class="fixed inset-y-4 left-4 z-50 w-[min(82vw,320px)] lg:hidden">
            <Sidebar />
          </div>
        </Show>

        <main class="flex min-h-[calc(100vh-2rem)] min-w-0 flex-1 flex-col gap-6 rounded-[34px] border border-white/6 bg-slate-950/32 p-4 shadow-[0_30px_120px_rgba(0,0,0,0.24)] sm:p-5 lg:p-6">
          <header class="glass-panel flex flex-col gap-4 rounded-[28px] border border-white/8 px-4 py-4 sm:px-5 lg:flex-row lg:items-center lg:justify-between">
            <div class="flex items-start gap-3">
              <Button
                class="lg:hidden"
                onClick={() => setMobileOpen(true)}
                size="icon"
                variant="outline"
              >
                <Menu class="size-4" />
              </Button>
              <div class="space-y-2">
                <div class="flex flex-wrap items-center gap-2">
                  <Badge variant="outline">Admin Dashboard</Badge>
                  <Badge variant="success">Realtime-ready</Badge>
                </div>
                <div>
                  <h1 class="text-lg font-semibold tracking-[-0.03em] text-foreground sm:text-xl">
                    {activeItem().label}
                  </h1>
                  <p class="text-sm text-muted-foreground">
                    {activeItem().hint} with a Tailwind and shadcn-style operator interface.
                  </p>
                </div>
              </div>
            </div>

            <div class="flex flex-wrap items-center gap-3">
              <div class="rounded-full border border-white/8 bg-white/5 px-4 py-2">
                <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-muted-foreground">
                  Cluster
                </p>
                <div class="mt-1 flex items-center gap-2 text-sm text-foreground">
                  <Activity class="size-4 text-accent" />
                  Dev shard stable
                </div>
              </div>
              <div class="rounded-full border border-white/8 bg-white/5 px-4 py-2">
                <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-muted-foreground">
                  Surface
                </p>
                <div class="mt-1 flex items-center gap-2 text-sm text-foreground">
                  <Cable class="size-4 text-primary" />
                  Phase 4 complete
                </div>
              </div>
              <Button class="hidden sm:inline-flex" variant="secondary">
                <Sparkles class="size-4" />
                Release Window
              </Button>
            </div>
          </header>

          <div class="min-w-0 flex-1">{props.children}</div>
        </main>
      </div>
    </div>
  );
}
