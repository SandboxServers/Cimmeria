import {
  createContext,
  useContext,
  useState,
  useEffect,
  type HTMLAttributes,
  type ButtonHTMLAttributes,
  type ReactNode,
} from 'react';
import { cn } from '../../lib/utils';

type TabsContextValue = {
  value: string;
  setValue: (value: string) => void;
};

const TabsContext = createContext<TabsContextValue | undefined>(undefined);

function useTabsContext() {
  const context = useContext(TabsContext);

  if (!context) {
    throw new Error('Tabs components must be used inside <Tabs>.');
  }

  return context;
}

type TabsProps = HTMLAttributes<HTMLDivElement> & {
  defaultValue: string;
  value?: string;
  onValueChange?: (value: string) => void;
  children?: ReactNode;
};

export function Tabs({ className, defaultValue, value, onValueChange, children, ...rest }: TabsProps) {
  const [internalValue, setInternalValue] = useState(value ?? defaultValue);

  useEffect(() => {
    if (value !== undefined) {
      setInternalValue(value);
    }
  }, [value]);

  const contextValue: TabsContextValue = {
    value: value ?? internalValue,
    setValue: (next) => {
      if (value === undefined) {
        setInternalValue(next);
      }

      onValueChange?.(next);
    },
  };

  return (
    <TabsContext.Provider value={contextValue}>
      <div className={cn('space-y-5', className)} {...rest}>
        {children}
      </div>
    </TabsContext.Provider>
  );
}

export function TabsList({ className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        'inline-flex w-full flex-wrap items-center gap-2 rounded-[24px] border border-white/8 bg-white/4 p-2',
        className,
      )}
      {...rest}
    />
  );
}

type TabsTriggerProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  value: string;
};

export function TabsTrigger({ className, value, children, ...rest }: TabsTriggerProps) {
  const context = useTabsContext();
  const isActive = context.value === value;

  return (
    <button
      aria-selected={isActive}
      className={cn(
        'inline-flex min-h-10 items-center justify-center rounded-full px-4 py-2 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground',
        isActive && 'bg-primary text-primary-foreground shadow-[0_8px_24px_rgba(245,170,49,0.24)]',
        className,
      )}
      data-state={isActive ? 'active' : 'inactive'}
      onClick={() => context.setValue(value)}
      type="button"
      {...rest}
    >
      {children}
    </button>
  );
}

type TabsContentProps = HTMLAttributes<HTMLDivElement> & {
  value: string;
  children?: ReactNode;
};

export function TabsContent({ className, value, children, ...rest }: TabsContentProps) {
  const context = useTabsContext();

  if (context.value !== value) {
    return null;
  }

  return (
    <div className={cn('space-y-5', className)} {...rest}>
      {children}
    </div>
  );
}
