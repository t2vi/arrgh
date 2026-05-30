import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { api, getToken } from '@/api'
import { ROUTES } from '@/lib/routes'

export interface SetupHandle {
  step: 1 | 2
  goToStep2: () => void
  finish: () => void
}

export function useSetup(): SetupHandle {
  const navigate = useNavigate()
  const [step, setStep] = useState<1 | 2>(1)

  useEffect(() => {
    if (!getToken()) return
    // Already have a valid session — setup is done, redirect to library.
    api.me().then(() => navigate(ROUTES.home, { replace: true })).catch(() => {})
  }, [navigate])

  return {
    step,
    goToStep2: () => setStep(2),
    finish: () => navigate(ROUTES.home, { replace: true }),
  }
}
