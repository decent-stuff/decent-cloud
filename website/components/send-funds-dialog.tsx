"use client";

import { Button } from "@/components/ui/button";
import { useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";

interface SendFundsDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSend: (recipient: string, amount: string) => void;
  tokenName: string;
  maxAmount?: number;
}

export function SendFundsDialog({
  isOpen,
  onClose,
  onSend,
  tokenName,
  maxAmount,
}: SendFundsDialogProps) {
  const [recipient, setRecipient] = useState("");
  const [amount, setAmount] = useState("");
  const [error, setError] = useState<string | null>(null);

  const handleSend = () => {
    // Basic validation
    if (!recipient.trim()) {
      setError("Recipient is required");
      return;
    }

    if (!amount.trim()) {
      setError("Amount is required");
      return;
    }

    const amountValue = parseFloat(amount);
    if (isNaN(amountValue) || amountValue <= 0) {
      setError("Please enter a valid amount greater than 0");
      return;
    }

    if (maxAmount !== undefined && amountValue > maxAmount) {
      setError(`Amount exceeds your balance of ${maxAmount} ${tokenName}`);
      return;
    }

    // Clear error and call onSend
    setError(null);
    onSend(recipient, amount);

    // Reset form
    setRecipient("");
    setAmount("");
    onClose();
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50" />
        <Dialog.Content className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 bg-gray-900 rounded-xl p-6 shadow-2xl w-[95vw] sm:w-[90vw] max-w-md border border-white/10 animate-in fade-in-0 zoom-in-95 z-50">
          <Dialog.Title className="text-2xl font-bold mb-4 text-white">
            Send {tokenName}
          </Dialog.Title>

          <div className="space-y-4">
            <div>
              <label
                htmlFor="recipient"
                className="block text-sm font-medium text-white/80 mb-1"
              >
                Recipient
              </label>
              <input
                id="recipient"
                type="text"
                value={recipient}
                onChange={(e) => setRecipient(e.target.value)}
                placeholder="Enter recipient address"
                className="w-full px-3 py-2 bg-gray-800 border border-white/10 rounded-md text-white placeholder-white/40 focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>

            <div>
              <label
                htmlFor="amount"
                className="block text-sm font-medium text-white/80 mb-1"
              >
                Amount
              </label>
              <div className="relative">
                <input
                  id="amount"
                  type="text"
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  placeholder="0.00"
                  className="w-full px-3 py-2 bg-gray-800 border border-white/10 rounded-md text-white placeholder-white/40 focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <div className="absolute inset-y-0 right-3 flex items-center pointer-events-none">
                  <span className="text-white/60">{tokenName}</span>
                </div>
              </div>
              {maxAmount !== undefined && (
                <div className="mt-1 text-xs text-white/60 flex justify-between">
                  <span>
                    Available: {maxAmount} {tokenName}
                  </span>
                  <button
                    type="button"
                    className="text-blue-400 hover:text-blue-300"
                    onClick={() => setAmount(maxAmount.toString())}
                  >
                    Max
                  </button>
                </div>
              )}
            </div>

            {error && (
              <div className="p-2 bg-red-900/30 border border-red-500/30 rounded text-red-400 text-sm">
                {error}
              </div>
            )}

            <div className="mt-6 flex justify-end space-x-4">
              <Button
                variant="outline"
                onClick={onClose}
                className="bg-transparent hover:bg-gray-800"
              >
                Cancel
              </Button>
              <Button
                onClick={handleSend}
                className="bg-blue-500 hover:bg-blue-600"
              >
                Send
              </Button>
            </div>
          </div>

          <Dialog.Close asChild>
            <button
              className="absolute top-4 right-4 p-2 rounded-full text-white/60 hover:text-white hover:bg-white/10 transition-all duration-200 group"
              aria-label="Close dialog"
            >
              <svg
                className="w-5 h-5 transform group-hover:rotate-90 transition-transform duration-200"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </Dialog.Close>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
