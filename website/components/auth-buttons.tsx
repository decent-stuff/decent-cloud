'use client'

import { useAuth } from '@/lib/auth-context'
import { Button } from '@/components/ui/button'
import { motion, AnimatePresence } from 'framer-motion'
import Link from 'next/link'
import { AuthDialog } from './auth-dialog'

export function AuthButtons() {
  const { isAuthenticated, principal, logout } = useAuth()

  if (isAuthenticated && principal) {
    return (
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -10 }}
        className="flex items-center gap-4"
      >
        <span className="text-sm text-white/80">
          <Link href="/dashboard" className="text-sm text-white hover:underline">
            Logged in: {principal.toText().slice(0, 5)}
          </Link>
        </span>
        <Link
          href="https://www.kongswap.io/swap?from=ryjl3-tyaaa-aaaaa-aaaba-cai&to=ggi4a-wyaaa-aaaai-actqq-cai"
          target="_blank"
          rel="noopener noreferrer"
        >
          <Button
            variant="outline"
            className="bg-gradient-to-r from-purple-500 to-blue-500 text-white hover:from-blue-500 hover:to-purple-500"
          >
            Top Up DCT
          </Button>
        </Link>
        <Button
          onClick={() => logout()}
          variant="outline"
          className="bg-white/10 text-white hover:bg-white/20"
        >
          Sign Out
        </Button>
      </motion.div>
    )
  }

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -10 }}
        className="flex gap-4"
      >
        <AuthDialog />
      </motion.div>
    </AnimatePresence>
  )
}
