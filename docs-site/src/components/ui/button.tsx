import { cn } from '@/lib/utils'
import type { ButtonHTMLAttributes } from 'react'

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'default' | 'outline' | 'ghost'
  size?: 'default' | 'sm' | 'lg'
}

export function Button({ className, variant = 'default', size = 'default', ...props }: Props) {
  return (
    <button
      className={cn(
        'inline-flex items-center justify-center gap-2 rounded-md font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50',
        variant === 'default' && 'bg-primary text-primary-foreground hover:bg-primary/90',
        variant === 'outline' && 'border border-border bg-transparent hover:bg-muted text-foreground',
        variant === 'ghost' && 'hover:bg-muted text-foreground',
        size === 'default' && 'h-9 px-4 py-2 text-sm',
        size === 'sm' && 'h-7 px-3 text-xs',
        size === 'lg' && 'h-11 px-6 text-base',
        className,
      )}
      {...props}
    />
  )
}
