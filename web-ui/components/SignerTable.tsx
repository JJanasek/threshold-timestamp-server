import { StatusSigner } from "@/lib/api";

interface SignerTableProps {
  signers: StatusSigner[];
}

export default function SignerTable({ signers }: SignerTableProps) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse">
        <thead>
          <tr className="border-b-2 border-dashed border-pencil/30">
            <th className="text-left py-2 px-3 font-[family-name:var(--font-kalam)]">ID</th>
            <th className="text-left py-2 px-3 font-[family-name:var(--font-kalam)]">Nostr Public Key</th>
          </tr>
        </thead>
        <tbody>
          {signers.map((s) => (
            <tr key={s.signer_id} className="border-b border-dashed border-pencil/10">
              <td className="py-2 px-3 font-bold">#{s.signer_id}</td>
              <td className="py-2 px-3 font-mono text-xs break-all">{s.npub}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
