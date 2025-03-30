"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  validateBlockchain,
  registerProvider,
  ValidationResult,
} from "@/lib/blockchain-validator";
import { ledgerService } from "@/lib/ledger-service";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faCheckCircle,
  faSync,
  faTriangleExclamation,
} from "@fortawesome/free-solid-svg-icons";
import { useAuth } from "@/lib/auth-context";

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
  defaultMemo = "Website validator",
  className = "",
  darkMode = false,
  title = "Blockchain Validator",
  description = "Validate the blockchain by checking in as a node provider",
  onValidationComplete,
  showHeader = true,
  renderAsCard = true,
}: BlockchainValidatorProps) {
  const { isAuthenticated, currentIdentity } = useAuth();
  const principal = currentIdentity?.principal;
  const [memo, setMemo] = useState<string>(defaultMemo);
  const [isValidating, setIsValidating] = useState<boolean>(false);
  const [isRegistering, setIsRegistering] = useState<boolean>(false);
  const [errorMessage, setError] = useState<string | undefined>();
  const [result, setResult] = useState<ValidationResult | null>(null);
  const [blockHash, setLastBlockHash] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [isProviderRegistered, setIsProviderRegistered] =
    useState<boolean>(false);

  // Fetch ledger entries on component mount
  useEffect(() => {
    let isMounted = true;

    refreshData().catch((err) => {
      if (isMounted) {
        console.error("Error fetching entries:", err);
        setError(
          err instanceof Error ? err.message : "Failed to fetch ledger entries"
        );
      }
    });

    // Clean up on unmount
    return () => {
      isMounted = false;
    };
  }, []);

  // Handle validation
  const handleValidate = async () => {
    try {
      setIsValidating(true);
      setResult(null);

      // Validate the blockchain
      const validationResult = await validateBlockchain(memo);
      setResult(validationResult);

      // Call the callback if provided
      if (onValidationComplete) {
        onValidationComplete(validationResult);
      }
    } catch (error: unknown) {
      console.error("Error during validation:", error);
      const errorResult = {
        success: false,
        message: `Unexpected error: ${
          error instanceof Error ? error.message : String(error)
        }`,
      };
      setResult(errorResult);

      if (onValidationComplete) {
        onValidationComplete(errorResult);
      }
    } finally {
      setIsValidating(false);
    }
  };

  // Handle registration
  const handleRegister = async () => {
    try {
      setIsRegistering(true);
      setResult(null);

      // Register as a provider
      const registrationResult = await registerProvider();
      setResult(registrationResult);

      // If registration was successful, refresh the data to update the isProviderRegistered state
      if (registrationResult.success) {
        await refreshData();
      }

      // Call the callback if provided
      if (onValidationComplete) {
        onValidationComplete(registrationResult);
      }
    } catch (error: unknown) {
      console.error("Error during registration:", error);
      const errorResult = {
        success: false,
        message: `Unexpected error during registration: ${
          error instanceof Error ? error.message : String(error)
        }`,
      };
      setResult(errorResult);

      if (onValidationComplete) {
        onValidationComplete(errorResult);
      }
    } finally {
      setIsRegistering(false);
    }
  };

  // Refresh blockchain data
  const refreshData = async () => {
    setIsLoading(true);
    try {
      // Get the parent block hash
      const hash = await ledgerService.getLastBlockHash();
      setLastBlockHash(hash);
      const isRegistered =
        (isAuthenticated &&
          principal &&
          (await ledgerService.isProviderRegistered(principal.toText()))) ||
        false;
      setIsProviderRegistered(isRegistered);
      setError(undefined);
    } catch (error) {
      console.error("Error refreshing blockchain data:", error);
      setError(
        error instanceof Error
          ? error.message
          : "Failed to refresh blockchain data"
      );
    } finally {
      setIsLoading(false);
    }
  };

  const content = (
    <>
      {isLoading ? (
        <div className={`text-center py-4 ${darkMode ? "text-white/70" : ""}`}>
          Loading latest block data...
        </div>
      ) : blockHash ? (
        <div className="space-y-4">
          <div className="space-y-2">
            <div className={`font-medium ${darkMode ? "text-white" : ""}`}>
              Latest Block Hash to Validate (confirm you have seen it and have a
              copy of it):
            </div>
            <div
              className={`p-3 ${
                darkMode
                  ? "bg-white/10 border border-white/20 text-white"
                  : "bg-gray-100"
              } rounded-md break-all text-sm font-mono`}
            >
              {blockHash}
            </div>
          </div>

          <div className="space-y-2">
            <div className={`font-medium ${darkMode ? "text-white" : ""}`}>
              Memo (optional)
            </div>
            <input
              value={memo}
              onChange={(e) => setMemo(e.target.value)}
              placeholder="Enter a memo for this validation"
              maxLength={32}
              disabled={isValidating}
              className={`w-full px-3 py-2 ${
                darkMode
                  ? "bg-white/10 border border-white/20 text-white"
                  : "border"
              } ${
                new TextEncoder().encode(memo).length > 32 ? "bg-red-500" : ""
              } rounded-md`}
            />
            <p
              className={`text-xs ${
                darkMode ? "text-white/70" : "text-gray-500"
              }`}
            >
              Max 32 bytes. Current length:{" "}
              {new TextEncoder().encode(memo).length} bytes
            </p>
          </div>
          {isProviderRegistered ? (
            <div
              className={`p-4 rounded-md ${
                darkMode
                  ? "bg-green-900/30 text-green-400"
                  : "bg-green-50 text-green-800"
              } mb-4`}
            >
              <div className="mt-1">
                Blockchain validation will proceed with the registered identity
                `{principal?.toText()}`.
              </div>
            </div>
          ) : (
            <div
              className={`p-4 rounded-md ${
                darkMode
                  ? "bg-amber-900/30 text-amber-400"
                  : "bg-amber-50 text-amber-800"
              } mb-4`}
            >
              <div className="font-semibold">
                <FontAwesomeIcon
                  icon={faTriangleExclamation}
                  className="mr-1 text-yellow-400"
                />
                Registration Required
              </div>
              <div className="mt-1">
                Your current identity `{principal?.toText()}` may not be
                registered as a provider. The validation may fail.
              </div>
            </div>
          )}
        </div>
      ) : errorMessage ? (
        <div
          className={`text-center py-4 ${
            darkMode ? "text-red-400" : "text-red-600"
          }`}
        >
          <p>Error: {errorMessage}</p>
          <Button
            onClick={refreshData}
            className={`mt-3 ${
              darkMode
                ? "bg-blue-600 hover:bg-blue-700 text-white"
                : "bg-blue-500 hover:bg-blue-600 text-white"
            }`}
          >
            <FontAwesomeIcon icon={faSync} className="mr-2" />
            Retry
          </Button>
        </div>
      ) : (
        <div
          className={`text-center py-4 ${
            darkMode ? "text-amber-400" : "text-amber-600"
          }`}
        >
          <p>
            No parent block hash found. The blockchain data may still be
            loading.
          </p>
          <p className="mt-2 text-sm">This could be due to:</p>
          <ul className="list-disc pl-5 mt-1 text-left text-sm">
            <li>The ledger service is still initializing</li>
            <li>No blocks have been fetched from the network yet</li>
            <li>Network connectivity issues</li>
          </ul>
          <Button
            onClick={refreshData}
            className={`mt-3 ${
              darkMode
                ? "bg-blue-600 hover:bg-blue-700 text-white"
                : "bg-blue-500 hover:bg-blue-600 text-white"
            }`}
          >
            <FontAwesomeIcon icon={faSync} className="mr-2" />
            Refresh Block Data
          </Button>
        </div>
      )}

      {result && (
        <div
          className={`p-4 rounded-md ${
            darkMode
              ? result.success
                ? "bg-green-900/30"
                : "bg-red-900/30"
              : result.success
              ? "bg-green-50 text-green-800"
              : "bg-red-50 text-red-800"
          } mt-4`}
        >
          <div className="flex items-center gap-2 font-semibold">
            {darkMode ? (
              <FontAwesomeIcon
                icon={faCheckCircle}
                className={result.success ? "text-green-400" : "text-red-400"}
              />
            ) : null}
            {result.success ? "✓ Success" : "✗ Error"}
          </div>
          <div className="mt-2">{result.message}</div>
        </div>
      )}
    </>
  );

  const buttons = (
    <div className="flex flex-col space-y-2">
      {!isProviderRegistered && (
        <Button
          onClick={handleRegister}
          disabled={isRegistering || !blockHash}
          className={`w-full ${
            darkMode
              ? "bg-green-600 hover:bg-green-700 text-white"
              : "bg-green-500 hover:bg-green-600 text-white"
          }`}
        >
          {isRegistering ? "Registering..." : "Register as Provider"}
        </Button>
      )}
      <Button
        onClick={handleValidate}
        disabled={isValidating || !blockHash}
        className={`w-full ${
          darkMode ? "bg-blue-600 hover:bg-blue-700 text-white" : ""
        }`}
        title={
          isProviderRegistered
            ? ""
            : "You may not be registered as a provider yet, validation may fail."
        }
      >
        {isValidating ? "Validating..." : "Validate Blockchain"}
      </Button>
    </div>
  );

  if (!renderAsCard) {
    return (
      <div className={className}>
        <div className="space-y-4">{content}</div>
        <div className="mt-4">{buttons}</div>
      </div>
    );
  }

  return (
    <Card
      className={`w-full ${
        darkMode ? "bg-white/10 backdrop-blur-sm border border-white/20" : ""
      } ${className}`}
    >
      {showHeader && (
        <CardHeader>
          <CardTitle className={darkMode ? "text-white" : ""}>
            {title}
          </CardTitle>
          <CardDescription className={darkMode ? "text-white/90" : ""}>
            {description}
          </CardDescription>
        </CardHeader>
      )}
      <CardContent className="space-y-4">{content}</CardContent>
      <CardFooter>{buttons}</CardFooter>
    </Card>
  );
}
