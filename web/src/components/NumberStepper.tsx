export function NumberStepper({ value, min, max, onChange }: {
  value: number
  min: number
  max: number
  onChange: (n: number) => void
}) {
  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        className="w-7 h-7 rounded-md bg-muted text-sm flex items-center justify-center hover:bg-accent disabled:opacity-40"
        disabled={value <= min}
        onClick={() => onChange(value - 1)}
      >−</button>
      <span className="w-5 text-center text-sm font-semibold tabular-nums">{value}</span>
      <button
        type="button"
        className="w-7 h-7 rounded-md bg-muted text-sm flex items-center justify-center hover:bg-accent disabled:opacity-40"
        disabled={value >= max}
        onClick={() => onChange(value + 1)}
      >+</button>
    </div>
  )
}
