"use client";

import { useState, useCallback } from "react";
import FileUpload from "@/components/FileUpload";
import HashInput from "@/components/HashInput";
import TokenDisplay from "@/components/TokenDisplay";
import { postTimestamp, TimestampToken } from "@/lib/api";

export default function SigningPage() {
  const [hash, setHash] = useState<string | null>(null);
  const [fileName, setFileName] = useState<string | null>(null);
  const [token, setToken] = useState<TimestampToken | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSign = useCallback(async (h: string) => {
    setHash(h);
    setError(null);
    setToken(null);
    setLoading(true);
    try {
      const result = await postTimestamp(h);
      setToken(result);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }, []);

  const handleFileHash = useCallback(
    (h: string, name: string) => {
      setFileName(name);
      handleSign(h);
    },
    [handleSign],
  );

  return (
    <div>
      <h1 className="font-[family-name:var(--font-kalam)] text-4xl font-bold mb-2">
        Timestamp a Document
      </h1>
      <p className="text-erased mb-8">
        Upload a file or paste a SHA-256 hash to get a threshold-signed timestamp token.
      </p>

      <div className="space-y-6">
        <div>
          <h2 className="font-[family-name:var(--font-kalam)] text-xl font-bold mb-3">
            Option 1: Upload a file
          </h2>
          <FileUpload onHash={handleFileHash} />
        </div>

        <div className="flex items-center gap-4">
          <hr className="flex-1 dashed-divider" />
          <span className="text-erased font-bold">OR</span>
          <hr className="flex-1 dashed-divider" />
        </div>

        <div>
          <h2 className="font-[family-name:var(--font-kalam)] text-xl font-bold mb-3">
            Option 2: Enter hash directly
          </h2>
          <HashInput onSubmit={handleSign} disabled={loading} />
        </div>
      </div>

      {hash && !token && !error && (
        <div className="mt-6 wobbly-md bg-marker-yellow/20 p-4">
          <p className="text-sm">
            {fileName && <><span className="font-bold">{fileName}</span> &rarr; </>}
            Signing hash: <code className="font-mono text-xs break-all">{hash}</code>
          </p>
          {loading && <p className="text-sm text-erased mt-1">Waiting for threshold signature...</p>}
        </div>
      )}

      {error && (
        <div className="mt-6 wobbly bg-marker-red/20 p-4">
          <p className="font-bold text-red-800">Error</p>
          <p className="text-sm">{error}</p>
        </div>
      )}

      {token && <TokenDisplay token={token} />}
    </div>
  );
}
