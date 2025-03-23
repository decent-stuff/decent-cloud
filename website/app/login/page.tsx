"use client";

import { useEffect, useState } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import { AuthDialog } from "@/components/auth-dialog";
import { useAuth } from "@/lib/auth-context";

export default function LoginPage() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const { isAuthenticated } = useAuth();
  const [returnUrl, setReturnUrl] = useState<string>("/dashboard");

  useEffect(() => {
    // Get the return URL from the query parameters or default to dashboard
    const returnParam = searchParams.get("returnUrl");
    if (returnParam) {
      setReturnUrl(decodeURIComponent(returnParam));
    }

    // If user is already authenticated, redirect to the return URL
    if (isAuthenticated) {
      router.push(returnUrl);
    }
  }, [isAuthenticated, router, searchParams, returnUrl]);

  return (
    <div className="min-h-screen flex flex-col items-center justify-center bg-gradient-to-b from-gray-900 to-gray-800">
      <div className="w-full max-w-md p-8 space-y-8 bg-white rounded-xl shadow-2xl">
        <div className="mt-8 flex justify-center">
          <AuthDialog autoOpen={true} returnUrl={returnUrl} />
        </div>
      </div>
    </div>
  );
}
