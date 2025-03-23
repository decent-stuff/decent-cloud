"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import { AuthDialog } from "@/components/auth-dialog";

function LoginContent() {
  const searchParams = useSearchParams();
  const [returnUrl, setReturnUrl] = useState<string>("/dashboard");

  useEffect(() => {
    // Get the return URL from the query parameters or default to dashboard
    const returnParam = searchParams.get("returnUrl");
    if (returnParam) {
      setReturnUrl(decodeURIComponent(returnParam));
    }
  }, [searchParams, returnUrl]);

  return (
    <div className="w-full max-w-md p-8 space-y-8 bg-white rounded-xl shadow-2xl">
      <div className="mt-8 flex justify-center">
        <AuthDialog autoOpen={true} returnUrl={returnUrl} />
      </div>
    </div>
  );
}

export default function LoginPage() {
  return (
    <div className="min-h-screen flex flex-col items-center justify-center bg-gradient-to-b from-gray-900 to-gray-800">
      <Suspense fallback={<div>Loading...</div>}>
        <LoginContent />
      </Suspense>
    </div>
  );
}
