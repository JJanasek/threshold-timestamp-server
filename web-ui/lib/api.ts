const API_URL =
  process.env.NEXT_PUBLIC_API_URL || "http://localhost:8000";

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
