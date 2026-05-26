interface Props {
  value: number // 0–1
}

export function ProgressBar({ value }: Props) {
  return (
    <div className="h-[3px] w-full bg-muted shrink-0">
      <div
        className="h-full bg-primary transition-[width] duration-150"
        style={{ width: `${Math.min(1, Math.max(0, value)) * 100}%` }}
      />
    </div>
  )
}
