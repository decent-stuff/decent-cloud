"use client";

import { useAuth } from "@/lib/auth-context";
import { Button } from "@/components/ui/button";
import { motion, AnimatePresence } from "framer-motion";
import Link from "next/link";
import { AuthDialog } from "./auth-dialog";

export function AuthButtons() {
  const { isAuthenticated, principal } = useAuth();

  if (isAuthenticated && principal) {
    return (
      <Link href="/dashboard">
        <Button
          variant="outline"
          className="bg-white/10 text-white hover:bg-white/20"
        >
          Dashboard
        </Button>
      </Link>
    );
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
  );
}
