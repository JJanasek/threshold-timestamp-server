"use client";

import { TimestampToken } from "@/lib/api";
import { DownloadIcon } from "./Icons";

interface TokenDisplayProps {
  token: TimestampToken;
}

export default function TokenDisplay({ token }: TokenDisplayProps) {
  const json = JSON.stringify(token, null, 2);
  const timestamp = new Date(token.timestamp * 1000).toLocaleString();

  const handleDownload = () => {
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `timestamp_${token.serial_number}.tst`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="tape-decoration wobbly-md bg-paper p-6 mt-6 shadow-hard">
      <h3 className="font-[family-name:var(--font-kalam)] text-xl font-bold mb-3">
        Timestamp Token #{token.serial_number}
      </h3>
      <p className="text-sm text-erased mb-3">Issued: {timestamp}</p>
      <pre className="notebook-lines bg-paper-dark/50 p-4 wobbly text-xs font-mono overflow-x-auto whitespace-pre-wrap break-all">
        {json}
      </pre>
      <button onClick={handleDownload} className="btn-hand-secondary mt-4 flex items-center gap-2">
        <DownloadIcon className="w-4 h-4" />
        Download .tst
      </button>
    </div>
  );
}
