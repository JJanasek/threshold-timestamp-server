"use client";

import { useEffect, useState } from "react";
import { getStatus, postDkg, StatusResponse } from "@/lib/api";
import StatusCard from "@/components/StatusCard";
import SignerTable from "@/components/SignerTable";

export default function AdminPage() {
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const [dkgStatus, setDkgStatus] = useState<"idle" | "running" | "success" | "error">("idle");
  const [dkgError, setDkgError] = useState<string | null>(null);
  const [dkgResult, setDkgResult] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function fetchStatus() {
      try {
        const data = await getStatus();
        if (!cancelled) setStatus(data);
      } catch (e) {
        if (!cancelled) setError((e as Error).message);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    fetchStatus();
    const interval = setInterval(fetchStatus, 5000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  async function handleDkg() {
    if (!confirm("This will run Distributed Key Generation. All signers must be online. Continue?")) return;
    setDkgStatus("running");
    setDkgError(null);
    try {
      const result = await postDkg();
      setDkgResult(result.group_public_key);
      setDkgStatus("success");
    } catch (e) {
      setDkgError((e as Error).message);
      setDkgStatus("error");
    }
  }

  return (
    <div>
      <h1 className="font-[family-name:var(--font-kalam)] text-4xl font-bold mb-2">
        Admin Dashboard
      </h1>
      <p className="text-erased mb-8">
        Real-time status of the threshold timestamp service.
      </p>

      {loading && <p className="text-erased">Loading status...</p>}

      {error && (
        <div className="wobbly bg-marker-red/20 p-4 mb-6">
          <p className="font-bold text-red-800">Connection Error</p>
          <p className="text-sm">{error}</p>
        </div>
      )}

      {status && (
        <>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-8">
            <StatusCard
              label="Health"
              value={status.healthy ? "Healthy" : "Unhealthy"}
              color={status.healthy ? "bg-marker-green" : "bg-marker-red"}
            />
            <StatusCard label="Active Sessions" value={status.active_sessions} />
            <StatusCard
              label="Threshold"
              value={`${status.k} of ${status.n}`}
              color="bg-marker-blue"
            />
            <StatusCard label="Signers" value={status.signers.length} color="bg-paper-dark" />
          </div>

          <div className="wobbly-md bg-paper p-6 shadow-hard mb-8">
            <h2 className="font-[family-name:var(--font-kalam)] text-xl font-bold mb-3">
              Key Ceremony (DKG)
            </h2>
            <p className="text-sm text-erased mb-4">
              Generate a new group key using Distributed Key Generation. All signer nodes must be online.
            </p>
            {!status.group_public_key && dkgStatus !== "success" && (
              <p className="text-sm text-red-700 font-bold mb-4">
                No group key configured &mdash; run DKG to initialize the system.
              </p>
            )}
            <button
              onClick={handleDkg}
              disabled={dkgStatus === "running"}
              className="btn-hand"
            >
              {dkgStatus === "running"
                ? "Running DKG..."
                : status.group_public_key
                  ? "Regenerate Group Key"
                  : "Run DKG"}
            </button>
            {dkgStatus === "success" && (
              <p className="mt-3 text-sm text-green-700">
                DKG complete! New group key: <span className="font-mono break-all">{dkgResult}</span>
              </p>
            )}
            {dkgStatus === "error" && (
              <p className="mt-3 text-sm text-red-700">DKG failed: {dkgError}</p>
            )}
          </div>

          <div className="wobbly-md bg-paper p-6 shadow-hard mb-8">
            <h2 className="font-[family-name:var(--font-kalam)] text-xl font-bold mb-1">
              Group Public Key
            </h2>
            <p className="font-mono text-xs break-all bg-paper-dark/50 p-3 wobbly">
              {status.group_public_key || "(not set — run DKG)"}
            </p>
          </div>

          <div className="wobbly-md bg-paper p-6 shadow-hard mb-8">
            <h2 className="font-[family-name:var(--font-kalam)] text-xl font-bold mb-3">
              Signer Nodes
            </h2>
            <SignerTable signers={status.signers} />
          </div>

          <div className="wobbly-md bg-paper p-6 shadow-hard">
            <h2 className="font-[family-name:var(--font-kalam)] text-xl font-bold mb-3">
              Relay URLs
            </h2>
            <ul className="space-y-1">
              {status.relay_urls.map((url) => (
                <li key={url} className="font-mono text-sm">
                  {url}
                </li>
              ))}
            </ul>
          </div>
        </>
      )}
    </div>
  );
}
