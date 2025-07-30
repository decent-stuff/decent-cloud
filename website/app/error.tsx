"use client";

import { useEffect } from "react";
import Link from "next/link";

export default function Error({
  error,
  reset,
}: {
  error: Error;
  reset: () => void;
}) {
  useEffect(() => {
    // Log the error to an error reporting service
    console.error("Application error:", error);
  }, [error]);

  return (
    <div className="min-h-screen flex flex-col items-center justify-center bg-gradient-to-b from-blue-900 to-black text-white p-4">
      <div className="text-center max-w-2xl">
        <h1 className="text-4xl font-bold mb-4">Something went wrong!</h1>
        <p className="text-xl mb-6 text-gray-300">
          We're sorry, but an unexpected error occurred.
        </p>
        <div className="mb-8 p-4 bg-red-900/30 rounded-lg">
          <p className="text-red-300 font-mono text-sm">
            {error.message || "An unknown error occurred"}
          </p>
        </div>
        <div className="flex flex-col sm:flex-row gap-4 justify-center">
          <button
            className="bg-blue-600 hover:bg-blue-700 text-white font-bold py-3 px-6 rounded-lg transition duration-300"
            onClick={() => reset()}
          >
            Try again
          </button>
          <Link 
            href="/" 
            className="inline-block bg-gray-600 hover:bg-gray-700 text-white font-bold py-3 px-6 rounded-lg transition duration-300 text-center"
          >
            Go Back Home
          </Link>
        </div>
      </div>
    </div>
  );
}