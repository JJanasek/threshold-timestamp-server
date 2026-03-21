"use client";

import { useState } from "react";

interface HashInputProps {
  onSubmit: (hash: string) => void;
  disabled?: boolean;
}

export default function HashInput({ onSubmit, disabled }: HashInputProps) {
  const [value, setValue] = useState("");

  const isValid = /^[0-9a-fA-F]{64}$/.test(value);

  return (
    <div className="flex gap-3">
      <input
        type="text"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder="Enter SHA-256 hash (64 hex chars)"
        className="flex-1 wobbly px-4 py-2 bg-paper font-mono text-sm focus:outline-none focus:ring-2 focus:ring-marker-yellow"
        maxLength={64}
      />
      <button
        onClick={() => onSubmit(value.toLowerCase())}
        disabled={!isValid || disabled}
        className="btn-hand disabled:opacity-40 disabled:cursor-not-allowed"
      >
        {disabled ? "Signing..." : "Timestamp"}
      </button>
    </div>
  );
}
