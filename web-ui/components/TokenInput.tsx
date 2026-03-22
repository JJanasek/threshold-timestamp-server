"use client";

import { useState, useCallback } from "react";
import { TimestampToken } from "@/lib/api";
import { UploadIcon } from "./Icons";

interface TokenInputProps {
  onSubmit: (token: TimestampToken) => void;
  disabled?: boolean;
}

export default function TokenInput({ onSubmit, disabled }: TokenInputProps) {
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);

  const tryParse = useCallback(
    (text: string) => {
      setError(null);
      try {
        const parsed = JSON.parse(text);
        if (
          typeof parsed.serial_number !== "number" ||
          typeof parsed.timestamp !== "number" ||
          typeof parsed.file_hash !== "string" ||
          typeof parsed.signature !== "string" ||
          typeof parsed.group_public_key !== "string"
        ) {
          throw new Error("Missing required fields");
        }
        onSubmit(parsed as TimestampToken);
      } catch (e) {
        setError(e instanceof SyntaxError ? "Invalid JSON" : (e as Error).message);
      }
    },
    [onSubmit],
  );

  const handleFile = useCallback(() => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".tst,.json";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      const text = await file.text();
      setValue(text);
      tryParse(text);
    };
    input.click();
  }, [tryParse]);

  return (
    <div>
      <textarea
        value={value}
        onChange={(e) => {
          setValue(e.target.value);
          setError(null);
        }}
        placeholder='Paste timestamp token JSON here...'
        rows={8}
        className="w-full wobbly px-4 py-3 bg-paper font-mono text-sm resize-y focus:outline-none focus:ring-2 focus:ring-marker-yellow"
      />
      {error && <p className="text-red-600 text-sm mt-1">{error}</p>}
      <div className="flex gap-3 mt-3">
        <button
          onClick={() => tryParse(value)}
          disabled={!value.trim() || disabled}
          className="btn-hand disabled:opacity-40 disabled:cursor-not-allowed"
        >
          {disabled ? "Verifying..." : "Verify Token"}
        </button>
        <button onClick={handleFile} className="btn-hand-secondary flex items-center gap-2">
          <UploadIcon className="w-4 h-4" />
          Upload .tst
        </button>
      </div>
    </div>
  );
}
