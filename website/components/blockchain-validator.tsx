'use client';

import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import {
  validateBlockchain,
  ValidationResult,
  getLatestParentBlockHash,
} from '@/lib/blockchain-validator';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faCheckCircle } from '@fortawesome/free-solid-svg-icons';

export interface BlockchainValidatorProps {
  /** Default memo text */
  defaultMemo?: string;
  /** Custom class name for the container */
  className?: string;
  /** Whether to use dark mode styling */
  darkMode?: boolean;
  /** Card title */
  title?: string;
  /** Card description */
  description?: string;
  /** Callback when validation completes */
  onValidationComplete?: (result: ValidationResult) => void;
  /** Whether to show the card header */
  showHeader?: boolean;
  /** Whether to render as a card or just the content */
  renderAsCard?: boolean;
}

export function BlockchainValidator({
  defaultMemo = 'Website validator',
  className = '',
  darkMode = false,
  title = 'Blockchain Validator',
  description = 'Validate the blockchain by checking in as a node provider',
  onValidationComplete,
  showHeader = true,
  renderAsCard = true
}: BlockchainValidatorProps) {
  const [memo, setMemo] = useState<string>(defaultMemo);
  const [isValidating, setIsValidating] = useState<boolean>(false);
  const [result, setResult] = useState<ValidationResult | null>(null);
  const [parentBlockHash, setParentBlockHash] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);

  // Fetch the parent block hash when the component loads
  useEffect(() => {
    // Use an IIFE (Immediately Invoked Function Expression) to properly await the Promise
    (async () => {
      try {
        setIsLoading(true);
        console.log('Fetching parent block hash...');

        // Try up to 3 times to get the parent block hash
        let hash = null;
        let attempts = 0;
        const maxAttempts = 3;

        while (!hash && attempts < maxAttempts) {
          attempts++;
          console.log(`Attempt ${attempts} to fetch parent block hash`);
          hash = await getLatestParentBlockHash();

          if (!hash && attempts < maxAttempts) {
            // Wait a bit before trying again
            await new Promise(resolve => setTimeout(resolve, 1000));
          }
        }

        if (hash) {
          console.log('Successfully fetched parent block hash:', hash);
          setParentBlockHash(hash);
        } else {
          console.error('Failed to fetch parent block hash after multiple attempts');
        }
      } catch (error) {
        console.error('Error fetching parent block hash:', error);
      } finally {
        setIsLoading(false);
      }
    })();
  }, []);

  const handleValidate = async () => {
    try {
      setIsValidating(true);
      setResult(null);

      // Validate the blockchain
      const validationResult = await validateBlockchain(memo);
      setResult(validationResult);

      // Update parent block hash if available
      if (validationResult.parentBlockHash) {
        setParentBlockHash(validationResult.parentBlockHash);
      } else {
        // Refresh the parent block hash after validation
        const hash = await getLatestParentBlockHash();
        setParentBlockHash(hash);
      }

      // Call the callback if provided
      if (onValidationComplete) {
        onValidationComplete(validationResult);
      }
    } catch (error: unknown) {
      console.error('Error during validation:', error);
      const errorResult = {
        success: false,
        message: `Unexpected error: ${error instanceof Error ? error.message : String(error)}`
      };
      setResult(errorResult);

      if (onValidationComplete) {
        onValidationComplete(errorResult);
      }
    } finally {
      setIsValidating(false);
    }
  };

  const content = (
    <>
      {isLoading ? (
        <div className={`text-center py-4 ${darkMode ? 'text-white/70' : ''}`}>
          Loading latest block data...
        </div>
      ) : parentBlockHash ? (
        <div className="space-y-4">
          <div className="space-y-2">
            <div className={`font-medium ${darkMode ? 'text-white' : ''}`}>Parent Block Hash</div>
            <div className={`p-3 ${darkMode ? 'bg-white/10 border border-white/20 text-white' : 'bg-gray-100'} rounded-md break-all text-sm font-mono`}>
              {parentBlockHash}
            </div>
            <p className={`text-xs ${darkMode ? 'text-white/70' : 'text-gray-500'}`}>
              This is the hash of the parent block you are validating
            </p>
          </div>

          <div className="space-y-2">
            <div className={`font-medium ${darkMode ? 'text-white' : ''}`}>Memo (optional)</div>
            <input
              value={memo}
              onChange={(e) => setMemo(e.target.value)}
              placeholder="Enter a memo for this validation"
              maxLength={32}
              disabled={isValidating}
              className={`w-full px-3 py-2 ${darkMode ? 'bg-white/10 border border-white/20 text-white' : 'border'} rounded-md`}
            />
            <p className={`text-xs ${darkMode ? 'text-white/70' : 'text-gray-500'}`}>
              Max 32 bytes. Current length: {new TextEncoder().encode(memo).length} bytes
            </p>
          </div>
        </div>
      ) : (
        <div className={`text-center py-4 ${darkMode ? 'text-amber-400' : 'text-amber-600'}`}>
          <p>No parent block hash found. The blockchain data may still be loading.</p>
          <p className="mt-2 text-sm">
            This could be due to:
            <ul className="list-disc pl-5 mt-1 text-left">
              <li>The ledger service is still initializing</li>
              <li>No blocks have been fetched from the network yet</li>
              <li>Network connectivity issues</li>
            </ul>
          </p>
          <button
            onClick={async () => {
              setIsLoading(true);
              try {
                const hash = await getLatestParentBlockHash();
                setParentBlockHash(hash);
              } catch (error) {
                console.error('Error refreshing parent block hash:', error);
              } finally {
                setIsLoading(false);
              }
            }}
            className={`mt-3 px-4 py-2 rounded-md ${darkMode ? 'bg-blue-600 hover:bg-blue-700 text-white' : 'bg-blue-500 hover:bg-blue-600 text-white'}`}
          >
            Refresh Block Data
          </button>
        </div>
      )}

      {result && (
        <div className={`p-4 rounded-md ${
          darkMode
            ? (result.success ? 'bg-green-900/30' : 'bg-red-900/30')
            : (result.success ? 'bg-green-50 text-green-800' : 'bg-red-50 text-red-800')
        } mt-4`}>
          <div className="flex items-center gap-2 font-semibold">
            {darkMode ? (
              <FontAwesomeIcon
                icon={faCheckCircle}
                className={result.success ? 'text-green-400' : 'text-red-400'}
              />
            ) : null}
            {result.success ? '✓ Success' : '✗ Error'}
          </div>
          <div className="mt-2">{result.message}</div>
        </div>
      )}
    </>
  );

  const button = (
    <Button
      onClick={handleValidate}
      disabled={isValidating || !parentBlockHash}
      className={`w-full ${darkMode ? 'bg-blue-600 hover:bg-blue-700 text-white' : ''}`}
    >
      {isValidating ? 'Validating...' : 'Validate Blockchain'}
    </Button>
  );

  if (!renderAsCard) {
    return (
      <div className={className}>
        <div className="space-y-4">
          {content}
        </div>
        <div className="mt-4">
          {button}
        </div>
      </div>
    );
  }

  return (
    <Card className={`w-full ${darkMode ? 'bg-white/10 backdrop-blur-sm border border-white/20' : ''} ${className}`}>
      {showHeader && (
        <CardHeader>
          <CardTitle className={darkMode ? 'text-white' : ''}>{title}</CardTitle>
          <CardDescription className={darkMode ? 'text-white/90' : ''}>
            {description}
          </CardDescription>
        </CardHeader>
      )}
      <CardContent className="space-y-4">
        {content}
      </CardContent>
      <CardFooter>
        {button}
      </CardFooter>
    </Card>
  );
}
