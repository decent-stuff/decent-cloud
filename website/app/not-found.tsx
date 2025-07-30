"use client";

import Link from "next/link";

export default function NotFound() {
  // Simple 404 page that doesn't rely on context providers
  return (
    <div className="min-h-screen flex flex-col items-center justify-center bg-gradient-to-b from-blue-900 to-black text-white p-4">
      <div className="text-center max-w-2xl">
        <h1 className="text-6xl font-bold mb-4">404</h1>
        <h2 className="text-3xl font-semibold mb-6">Page Not Found</h2>
        <p className="text-xl mb-8 text-gray-300">
          Sorry, the page you're looking for doesn't exist.
        </p>
        <Link 
          href="/" 
          className="inline-block bg-blue-600 hover:bg-blue-700 text-white font-bold py-3 px-6 rounded-lg transition duration-300"
        >
          Go Back Home
        </Link>
      </div>
    </div>
  );
}