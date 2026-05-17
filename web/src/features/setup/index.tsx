import { cn } from '@/lib/utils'
import { useSetup } from './hooks/useSetup'
import { StepAccount } from './components/StepAccount'
import { StepSettings } from './components/StepSettings'

export default function Setup() {
  const h = useSetup()

  return (
    <div className="min-h-screen bg-background flex items-center justify-center p-4">
      <div className="w-full max-w-sm space-y-8">
        <div className="text-center space-y-2">
          <h1 className="text-4xl font-black tracking-tight text-primary">*ARRgh</h1>
          {h.step === 1 ? (
            <>
              <p className="text-sm font-semibold">Welcome — create your account</p>
              <p className="text-xs text-muted-foreground">One-time setup for your self-hosted library.</p>
            </>
          ) : (
            <>
              <p className="text-sm font-semibold">Configure your library</p>
              <p className="text-xs text-muted-foreground">These can be changed later in Settings.</p>
            </>
          )}
        </div>

        <div className="flex items-center justify-center gap-2">
          {([1, 2] as const).map((s) => (
            <div key={s} className={cn('w-2 h-2 rounded-full transition-colors', h.step >= s ? 'bg-primary' : 'bg-muted')} />
          ))}
        </div>

        {h.step === 1
          ? <StepAccount onDone={h.goToStep2} />
          : <StepSettings onDone={h.finish} />
        }
      </div>
    </div>
  )
}
