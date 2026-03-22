const API_URL =
  process.env.NEXT_PUBLIC_API_URL || "http://localhost:8000";

const COLLECTOR_URL =
  process.env.NEXT_PUBLIC_COLLECTOR_URL || "http://localhost:9000";

export interface TimestampToken {
  serial_number: number;
  timestamp: number;
  file_hash: string;
  signature: string;
  group_public_key: string;
}

export interface StatusSigner {
  signer_id: number;
  npub: string;
}

export interface StatusResponse {
  healthy: boolean;
  active_sessions: number;
  k: number;
  n: number;
  group_public_key: string;
  signers: StatusSigner[];
  relay_urls: string[];
}

export interface PubkeyResponse {
  group_public_key: string;
  k: number;
  n: number;
  coordinator_npub: string;
}

export interface VerifyResponse {
  valid: boolean;
}

async function apiFetch<T>(
  path: string,
  init?: RequestInit,
): Promise<T> {
  const res = await fetch(`${API_URL}${path}`, {
    ...init,
    headers: { "Content-Type": "application/json", ...init?.headers },
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`API error ${res.status}: ${text}`);
  }
  return res.json();
}

export function getStatus(): Promise<StatusResponse> {
  return apiFetch("/api/v1/status");
}

export function getPubkey(): Promise<PubkeyResponse> {
  return apiFetch("/api/v1/pubkey");
}

export function postTimestamp(hash: string): Promise<TimestampToken> {
  return apiFetch("/api/v1/timestamp", {
    method: "POST",
    body: JSON.stringify({ hash }),
  });
}

export function postVerify(token: TimestampToken): Promise<VerifyResponse> {
  return apiFetch("/api/v1/verify", {
    method: "POST",
    body: JSON.stringify({ token }),
  });
}

export interface DkgResponse {
  group_public_key: string;
  success: boolean;
}

export function postDkg(): Promise<DkgResponse> {
  return apiFetch("/api/v1/dkg", { method: "POST" });
}

// -- Collector API ------------------------------------------------------------

export interface CollectorEvent {
  node_name: string;
  session_id: string | null;
  message: string;
  timestamp: number;
}

export async function getEvents(params?: {
  node_name?: string;
  session_id?: string;
}): Promise<CollectorEvent[]> {
  const query = new URLSearchParams();
  if (params?.node_name) query.set("node_name", params.node_name);
  if (params?.session_id) query.set("session_id", params.session_id);
  const qs = query.toString();
  const url = `${COLLECTOR_URL}/api/v1/events${qs ? `?${qs}` : ""}`;
  const res = await fetch(url);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Collector error ${res.status}: ${text}`);
  }
  return res.json();
}
