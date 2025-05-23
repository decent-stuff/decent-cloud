"use client";

import * as Dialog from "@radix-ui/react-dialog";
import { Button } from "@/components/ui/button";
import { useEffect, useState } from "react";
import { generateNewSeedPhrase } from "@/lib/auth-context";

interface SeedPhraseDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit?: (phrase: string) => void;
  withSeedPhrase?: string | null;
  mode?: 'backup' | 'login';
}

export function SeedPhraseDialog({
  isOpen,
  onClose,
  onSubmit,
  withSeedPhrase: withSeedPhrase = null,
  mode = 'login'
}: SeedPhraseDialogProps) {
  const [seedPhrase, setSeedPhrase] = useState<string>("");
  const [error, setError] = useState<string>("");
  const [isGenerating, setIsGenerating] = useState(false);

  useEffect(() => {
    if (!isOpen) {
      setSeedPhrase("");
      setError("");
      setIsGenerating(false);
    } else if (withSeedPhrase && mode === 'backup') {
      setSeedPhrase(withSeedPhrase);
      setIsGenerating(false);
    }
  }, [isOpen, withSeedPhrase, mode]);

  const handleGenerateNew = () => {
    const newPhrase = generateNewSeedPhrase();
    setSeedPhrase(newPhrase);
    setIsGenerating(true);
    setError("");
  };

  const handleConfirm = () => {
    const trimmedSeedPhrase = seedPhrase.trim();
    console.info("Confirming seed phrase:", trimmedSeedPhrase);
    if (!trimmedSeedPhrase) {
      setError("Please enter your seed phrase");
      return;
    }

    // Basic validation - check if it's a valid mnemonic (12 or 24 words)
    const wordCount = trimmedSeedPhrase.split(/\s+/).length;
    if (wordCount !== 12 && wordCount !== 24) {
      setError("Invalid seed phrase. Must be 12 or 24 words");
      return;
    }

    try {
      // Store seed phrase in the seed_phrases array
      const storedSeedPhrases = JSON.parse(localStorage.getItem("seed_phrases") || "[]");
      if (!storedSeedPhrases.includes(trimmedSeedPhrase)) {
        storedSeedPhrases.push(trimmedSeedPhrase);
        localStorage.setItem("seed_phrases", JSON.stringify(storedSeedPhrases));
      }

      onSubmit?.(trimmedSeedPhrase);
      onClose();
    } catch (err) {
      setError(`Invalid seed phrase format: ${err}`);
    }
  };

  const handleInput = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setSeedPhrase(e.target.value);
    setError("");
    setIsGenerating(false);
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={onClose}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50" />
        <Dialog.Content
          className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 bg-white rounded-lg p-4 sm:p-6 shadow-xl w-[95vw] sm:w-[90vw] max-w-md z-50"
          aria-describedby="dialog-description"
        >
          <Dialog.Title className="text-lg sm:text-xl font-semibold mb-3 sm:mb-4 text-gray-800">
            {mode === 'backup' ? 'Your Backup Seed Phrase' : 'Seed Phrase Authentication (Login/Register)'}
          </Dialog.Title>

          <div id="dialog-description" className="space-y-3 sm:space-y-4">
            {mode === 'login' && (
              <div className="flex gap-2 sm:gap-4 mb-4 sm:mb-6">
                <Button
                  onClick={handleGenerateNew}
                  className="flex-1 bg-blue-500 text-white hover:bg-blue-600 h-12 sm:h-10 text-sm sm:text-base"
                >
                  Generate New
                </Button>
                <Button
                  onClick={() => {
                    setSeedPhrase("");
                    setIsGenerating(false);
                  }}
                  className="flex-1 bg-gray-400 text-white hover:bg-gray-600 h-12 sm:h-10 text-sm sm:text-base"
                  disabled={!isGenerating}
                >
                  Enter Existing
                </Button>
              </div>
            )}

            {mode === 'backup' || isGenerating ? (
              <>
                <p className="text-sm sm:text-base text-gray-600">
                  {mode === 'backup' 
                    ? "This is your seed phrase for this identity. Store it safely to recover your account if needed." 
                    : "This is your new seed phrase. Write it down and keep it safe. You'll need it to recover your account."}
                </p>

                <div className="bg-gray-50 p-3 sm:p-4 rounded break-all">
                  <p className="font-mono text-sm sm:text-base text-gray-800">
                    {seedPhrase}
                  </p>
                </div>

                <div className="bg-red-50 p-3 sm:p-4 rounded border-2 border-red-500">
                  <p className="text-xs sm:text-sm">
                    <span className="font-bold text-red-600">
                      ⚠️ WARNING! ⚠️ If you lose this seed phrase, you will{" "}
                      <span className="underline">PERMANENTLY LOSE ACCESS</span>{" "}
                      to your account and all associated funds. This is your{" "}
                      <span className="underline">ONLY</span> recovery key.
                      Write it down and keep it safe! 🔒
                    </span>
                  </p>
                </div>
              </>
            ) : (
              <p className="text-sm sm:text-base text-gray-600">
                Enter an existing seed phrase to access your account. Using the
                same seed phrase as in the CLI will give you the same identity
                across both platforms.
              </p>
            )}

            {!isGenerating && mode === 'login' && (
              <textarea
                value={seedPhrase}
                onChange={handleInput}
                className="w-full h-28 sm:h-32 p-3 sm:p-4 border rounded font-mono text-sm sm:text-base text-gray-800 resize-none focus:outline-none focus:ring-2 focus:ring-blue-500 bg-gray-50"
                placeholder="Enter your 12 or 24 word seed phrase..."
              />
            )}

            {error && (
              <p className="text-xs sm:text-sm text-red-600">{error}</p>
            )}

            <Button
              onClick={mode === 'backup' ? onClose : handleConfirm}
              className="w-full bg-emerald-500 text-white hover:bg-emerald-600 h-14 sm:h-12 text-sm sm:text-base"
              disabled={mode === 'login' && !seedPhrase.trim()}
            >
              {mode === 'backup'
                ? "I have Safely Stored My Recovery Phrase"
                : isGenerating
                ? "I have Saved My Seed Phrase"
                : "Login/Register with Seed Phrase"}
            </Button>
          </div>

          <Dialog.Close asChild>
            <button
              className="absolute top-3 sm:top-4 right-3 sm:right-4 p-2 rounded-full text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-all duration-200 touch-manipulation"
              aria-label="Close"
            >
              <svg
                className="w-6 h-6 sm:w-5 sm:h-5"
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
