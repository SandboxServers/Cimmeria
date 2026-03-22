import type { ButtonHTMLAttributes } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../../lib/utils';

export const buttonVariants = cva(
  'inline-flex items-center justify-center gap-2 whitespace-nowrap text-sm font-medium transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/70 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 shrink-0',
  {
    variants: {
      variant: {
        default:
          'bg-primary-container text-on-primary bevel-stone shadow-[0_10px_30px_rgba(242,202,80,0.15)] hover:brightness-110',
        secondary:
          'bg-secondary-container text-secondary-foreground bevel-stone hover:brightness-110',
        outline:
          'border border-[rgba(77,70,53,0.15)] bg-surface-high text-on-surface hover:border-primary/40 hover:bg-surface-highest',
        ghost: 'text-on-surface-variant hover:bg-surface-highest hover:text-on-surface',
        destructive: 'bg-error text-surface hover:bg-error/90',
      },
      size: {
        default: 'h-11 px-5 py-2.5',
        sm: 'h-9 px-4 text-xs',
        lg: 'h-12 px-6 text-base',
        icon: 'size-10',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  },
);

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> &
  VariantProps<typeof buttonVariants>;

export function Button({ className, variant, size, type = 'button', ...rest }: ButtonProps) {
  return (
    <button
      className={cn(buttonVariants({ variant, size }), className)}
      type={type}
      {...rest}
    />
  );
}
