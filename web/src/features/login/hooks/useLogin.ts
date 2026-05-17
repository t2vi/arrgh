import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { api, setToken } from '@/api'
import { ROUTES } from '@/lib/routes'

export interface LoginHandle {
  username: string
  setUsername: (v: string) => void
  password: string
  setPassword: (v: string) => void
  error: string
  loading: boolean
  handleSubmit: (e: React.FormEvent) => void
}

export function useLogin(): LoginHandle {
  const navigate = useNavigate()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setError('')
    setLoading(true)
    try {
      const res = await api.login(username.trim(), password)
      setToken(res.token, res.username, res.role, res.allow_explicit)
      navigate(ROUTES.home, { replace: true })
    } catch {
      setError('Invalid username or password.')
    } finally {
      setLoading(false)
    }
  }

  return { username, setUsername, password, setPassword, error, loading, handleSubmit }
}
