import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { ROUTES } from '@/lib/routes'

export interface SetupHandle {
  step: 1 | 2
  goToStep2: () => void
  finish: () => void
}

export function useSetup(): SetupHandle {
  const navigate = useNavigate()
  const [step, setStep] = useState<1 | 2>(1)

  return {
    step,
    goToStep2: () => setStep(2),
    finish: () => navigate(ROUTES.home, { replace: true }),
  }
}
