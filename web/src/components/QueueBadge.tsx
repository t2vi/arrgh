import { useState, useEffect, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { Download } from 'lucide-react'
import { api, type QueueItem } from '@/api'
import { Button } from '@/components/ui/button'
import { ROUTES } from '@/lib/routes'

export default function QueueBadge() {
  const navigate = useNavigate()
  const [data, setData] = useState<QueueItem[]>([])

  const fetchQueue = useCallback(() => {
    api.getQueue().then(setData).catch(() => {})
  }, [])

  useEffect(() => {
    fetchQueue()
    const id = setInterval(fetchQueue, 3000)
    return () => clearInterval(id)
  }, [fetchQueue])

  const active = data.filter((i) => i.status === 'pending' || i.status === 'downloading')

  if (active.length === 0) return null

  return (
    <Button
      variant="ghost"
      size="icon"
      className="relative"
      onClick={() => navigate(ROUTES.queue)}
      title={`${active.length} download${active.length > 1 ? 's' : ''} queued`}
    >
      <Download className="w-4 h-4" />
      <span className="absolute -top-1 -right-1 bg-primary text-primary-foreground text-[9px] font-bold rounded-full min-w-[14px] h-[14px] flex items-center justify-center px-0.5">
        {active.length}
      </span>
    </Button>
  )
}
