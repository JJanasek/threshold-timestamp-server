"use client";

import { useState, useCallback } from "react";
import TokenInput from "@/components/TokenInput";
import { postVerify, TimestampToken } from "@/lib/api";
import { CheckCircleIcon, XCircleIcon } from "@/components/Icons";

export default function VerifyPage() {
  const [result, setResult] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleVerify = useCallback(async (token: TimestampToken) => {
    setResult(null);
    setError(null);
    setLoading(true);
    try {
      const res = await postVerify(token);
      setResult(res.valid);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }, []);

  return (
    <div>
      <h1 className="font-[family-name:var(--font-kalam)] text-4xl font-bold mb-2">
        Verify a Token
      </h1>
      <p className="text-erased mb-8">
        Paste or upload a timestamp token (.tst) to verify its signature.
      </p>

      <TokenInput onSubmit={handleVerify} disabled={loading} />

      {result !== null && (
        <div
          className={`mt-6 wobbly-md p-6 flex items-center gap-4 ${
            result ? "bg-marker-green/30" : "bg-marker-red/30"
          }`}
        >
          {result ? (
            <>
              <CheckCircleIcon className="w-10 h-10 text-green-700" />
              <div>
                <p className="font-[family-name:var(--font-kalam)] text-2xl font-bold text-green-800">
                  Valid
                </p>
                <p className="text-sm text-green-900/70">
                  The timestamp token signature is valid.
                </p>
              </div>
            </>
          ) : (
            <>
              <XCircleIcon className="w-10 h-10 text-red-700" />
              <div>
                <p className="font-[family-name:var(--font-kalam)] text-2xl font-bold text-red-800">
                  Invalid
                </p>
                <p className="text-sm text-red-900/70">
                  The timestamp token signature could not be verified.
                </p>
              </div>
            </>
          )}
        </div>
      )}

      {error && (
        <div className="mt-6 wobbly bg-marker-red/20 p-4">
          <p className="font-bold text-red-800">Error</p>
          <p className="text-sm">{error}</p>
        </div>
      )}
    </div>
  );
}
