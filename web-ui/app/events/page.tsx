"use client";

import { useCallback, useEffect, useState } from "react";
import { getEvents, CollectorEvent } from "@/lib/api";

const NODE_OPTIONS = ["all", "coordinator", "signer-1", "signer-2", "signer-3"];

function formatTime(epoch: number): string {
  if (!epoch) return "-";
  return new Date(epoch * 1000).toLocaleTimeString();
}

function formatDate(epoch: number): string {
  if (!epoch) return "";
  return new Date(epoch * 1000).toLocaleDateString();
}

function nodeColor(name: string): string {
  if (name === "coordinator") return "bg-marker-blue/30";
  if (name.startsWith("signer")) return "bg-marker-green/30";
  return "bg-paper-dark";
}

export default function EventsPage() {
  const [events, setEvents] = useState<CollectorEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [nodeFilter, setNodeFilter] = useState("all");
  const [sessionFilter, setSessionFilter] = useState("");
  const [autoRefresh, setAutoRefresh] = useState(true);

  const fetchEvents = useCallback(async () => {
    try {
      const params: { node_name?: string; session_id?: string } = {};
      if (nodeFilter !== "all") params.node_name = nodeFilter;
      if (sessionFilter.trim()) params.session_id = sessionFilter.trim();
      const data = await getEvents(params);
      setEvents(data);
      setError(null);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }, [nodeFilter, sessionFilter]);

  useEffect(() => {
    fetchEvents();
    if (!autoRefresh) return;
    const interval = setInterval(fetchEvents, 3000);
    return () => clearInterval(interval);
  }, [fetchEvents, autoRefresh]);

  const sessionIds = Array.from(
    new Set(events.map((e) => e.session_id).filter(Boolean) as string[])
  );

  return (
    <div>
      <h1 className="font-[family-name:var(--font-kalam)] text-4xl font-bold mb-2">
        Event Log
      </h1>
      <p className="text-erased mb-6">
        Unified audit timeline from all nodes.
      </p>

      {/* Filters */}
      <div className="wobbly-md bg-paper p-4 shadow-hard-sm mb-6 flex flex-wrap gap-4 items-end">
        <div>
          <label className="block text-sm font-bold mb-1">Node</label>
          <select
            value={nodeFilter}
            onChange={(e) => setNodeFilter(e.target.value)}
            className="wobbly px-3 py-1.5 bg-paper-dark text-sm font-mono"
          >
            {NODE_OPTIONS.map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </div>

        <div className="flex-1 min-w-[200px]">
          <label className="block text-sm font-bold mb-1">Session ID</label>
          <input
            type="text"
            value={sessionFilter}
            onChange={(e) => setSessionFilter(e.target.value)}
            placeholder="filter by session UUID..."
            className="wobbly w-full px-3 py-1.5 bg-paper-dark text-sm font-mono"
          />
        </div>

        <label className="flex items-center gap-2 text-sm cursor-pointer">
          <input
            type="checkbox"
            checked={autoRefresh}
            onChange={(e) => setAutoRefresh(e.target.checked)}
            className="w-4 h-4"
          />
          Auto-refresh
        </label>

        <button onClick={fetchEvents} className="btn-hand-secondary text-sm px-3 py-1.5">
          Refresh
        </button>
      </div>

      {error && (
        <div className="wobbly bg-marker-red/20 p-4 mb-6">
          <p className="font-bold text-red-800">Collector Error</p>
          <p className="text-sm">{error}</p>
          <p className="text-xs text-erased mt-1">
            Make sure the collector service is running on port 9000.
          </p>
        </div>
      )}

      {loading && <p className="text-erased">Loading events...</p>}

      {!loading && !error && events.length === 0 && (
        <div className="wobbly bg-paper-dark/50 p-6 text-center">
          <p className="text-erased">No events yet.</p>
          <p className="text-sm text-erased mt-1">
            Events will appear here when signing sessions are triggered.
          </p>
        </div>
      )}

      {events.length > 0 && (
        <>
          <p className="text-sm text-erased mb-3">
            {events.length} event{events.length !== 1 ? "s" : ""}
            {sessionIds.length > 0 &&
              ` across ${sessionIds.length} session${sessionIds.length !== 1 ? "s" : ""}`}
          </p>

          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b-2 border-dashed border-pencil/30 text-left">
                  <th className="py-2 px-3 font-bold">Time</th>
                  <th className="py-2 px-3 font-bold">Node</th>
                  <th className="py-2 px-3 font-bold">Message</th>
                  <th className="py-2 px-3 font-bold">Session</th>
                </tr>
              </thead>
              <tbody>
                {[...events].reverse().map((evt, i) => (
                  <tr
                    key={i}
                    className="border-b border-dashed border-pencil/10 hover:bg-paper-dark/30 transition-colors"
                  >
                    <td className="py-2 px-3 font-mono text-xs whitespace-nowrap text-erased">
                      <span title={formatDate(evt.timestamp)}>
                        {formatTime(evt.timestamp)}
                      </span>
                    </td>
                    <td className="py-2 px-3">
                      <span
                        className={`inline-block px-2 py-0.5 text-xs font-bold wobbly ${nodeColor(evt.node_name)}`}
                      >
                        {evt.node_name}
                      </span>
                    </td>
                    <td className="py-2 px-3">{evt.message}</td>
                    <td className="py-2 px-3 font-mono text-xs text-erased">
                      {evt.session_id ? (
                        <button
                          onClick={() => setSessionFilter(evt.session_id!)}
                          className="hover:text-pen underline decoration-dashed"
                          title="Click to filter by this session"
                        >
                          {evt.session_id.slice(0, 8)}...
                        </button>
                      ) : (
                        "-"
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  );
}
