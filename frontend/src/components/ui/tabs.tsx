import {
  type Accessor,
  createContext,
  createEffect,
  createSignal,
  type JSX,
  Show,
  splitProps,
  useContext,
} from 'solid-js';
import { cn } from '../../lib/utils';

type TabsContextValue = {
  value: Accessor<string>;
  setValue: (value: string) => void;
};

const TabsContext = createContext<TabsContextValue>();

function useTabsContext() {
  const context = useContext(TabsContext);

  if (!context) {
    throw new Error('Tabs components must be used inside <Tabs>.');
  }

  return context;
}

type TabsProps = JSX.HTMLAttributes<HTMLDivElement> & {
  defaultValue: string;
  value?: string;
  onValueChange?: (value: string) => void;
};

export function Tabs(props: TabsProps) {
  const [local, others] = splitProps(props, [
    'class',
    'defaultValue',
    'value',
    'onValueChange',
    'children',
  ]);
  const [internalValue, setInternalValue] = createSignal(
    local.value ?? local.defaultValue,
  );

  createEffect(() => {
    if (local.value !== undefined) {
      setInternalValue(local.value);
    }
  });

  const context: TabsContextValue = {
    value: () => local.value ?? internalValue(),
    setValue: (next) => {
      if (local.value === undefined) {
        setInternalValue(next);
      }

      local.onValueChange?.(next);
    },
  };

  return (
    <TabsContext.Provider value={context}>
      <div class={cn('space-y-5', local.class)} {...others}>
        {local.children}
      </div>
    </TabsContext.Provider>
  );
}

export function TabsList(props: JSX.HTMLAttributes<HTMLDivElement>) {
  const [local, others] = splitProps(props, ['class']);

  return (
    <div
      class={cn(
        'inline-flex w-full flex-wrap items-center gap-2 rounded-[24px] border border-white/8 bg-white/4 p-2',
        local.class,
      )}
      {...others}
    />
  );
}

type TabsTriggerProps = JSX.ButtonHTMLAttributes<HTMLButtonElement> & {
  value: string;
};

export function TabsTrigger(props: TabsTriggerProps) {
  const context = useTabsContext();
  const [local, others] = splitProps(props, ['class', 'value', 'children']);
  const isActive = () => context.value() === local.value;

  return (
    <button
      aria-selected={isActive()}
      class={cn(
        'inline-flex min-h-10 items-center justify-center rounded-full px-4 py-2 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground',
        isActive() && 'bg-primary text-primary-foreground shadow-[0_8px_24px_rgba(245,170,49,0.24)]',
        local.class,
      )}
      data-state={isActive() ? 'active' : 'inactive'}
      onClick={() => context.setValue(local.value)}
      type="button"
      {...others}
    >
      {local.children}
    </button>
  );
}

type TabsContentProps = JSX.HTMLAttributes<HTMLDivElement> & {
  value: string;
};

export function TabsContent(props: TabsContentProps) {
  const context = useTabsContext();
  const [local, others] = splitProps(props, ['class', 'value', 'children']);

  return (
    <Show when={context.value() === local.value}>
      <div class={cn('space-y-5', local.class)} {...others}>
        {local.children}
      </div>
    </Show>
  );
}
