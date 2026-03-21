"use client";

import { useState, useCallback } from "react";
import { UploadIcon } from "./Icons";

interface FileUploadProps {
  onHash: (hash: string, fileName: string) => void;
}

export default function FileUpload({ onHash }: FileUploadProps) {
  const [dragging, setDragging] = useState(false);
  const [fileName, setFileName] = useState<string | null>(null);
  const [hashing, setHashing] = useState(false);

  const hashFile = useCallback(
    async (file: File) => {
      setHashing(true);
      setFileName(file.name);
      try {
        const buffer = await file.arrayBuffer();
        const hashBuffer = await crypto.subtle.digest("SHA-256", buffer);
        const hashArray = Array.from(new Uint8Array(hashBuffer));
        const hex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
        onHash(hex, file.name);
      } finally {
        setHashing(false);
      }
    },
    [onHash],
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragging(false);
      const file = e.dataTransfer.files[0];
      if (file) hashFile(file);
    },
    [hashFile],
  );

  return (
    <div
      onDragOver={(e) => {
        e.preventDefault();
        setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={handleDrop}
      className={`tack-decoration wobbly p-8 text-center transition-colors cursor-pointer ${
        dragging ? "bg-marker-yellow/30" : "bg-paper-dark/50 hover:bg-paper-dark"
      }`}
      onClick={() => {
        const input = document.createElement("input");
        input.type = "file";
        input.onchange = () => {
          const file = input.files?.[0];
          if (file) hashFile(file);
        };
        input.click();
      }}
    >
      <UploadIcon className="w-10 h-10 mx-auto mb-3 text-erased" />
      {hashing ? (
        <p className="text-lg">Hashing...</p>
      ) : fileName ? (
        <p className="text-lg">
          <span className="font-bold">{fileName}</span> selected
        </p>
      ) : (
        <>
          <p className="text-lg font-bold">Drop a file here</p>
          <p className="text-erased text-sm mt-1">or click to browse</p>
        </>
      )}
    </div>
  );
}
