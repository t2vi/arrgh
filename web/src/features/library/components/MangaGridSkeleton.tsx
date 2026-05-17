import { Skeleton } from '@/components/ui/skeleton'

export function MangaGridSkeleton() {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(160px,1fr))] gap-5">
      {Array.from({ length: 12 }).map((_, i) => (
        <div key={i}>
          <Skeleton className="w-full aspect-[2/3] rounded-xl" />
          <Skeleton className="h-3.5 mt-2.5 w-3/4 rounded" />
          <Skeleton className="h-3 mt-1.5 w-1/2 rounded" />
        </div>
      ))}
    </div>
  )
}
