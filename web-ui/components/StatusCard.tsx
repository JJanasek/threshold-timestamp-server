interface StatusCardProps {
  label: string;
  value: string | number;
  color?: string;
}

export default function StatusCard({ label, value, color = "bg-marker-yellow" }: StatusCardProps) {
  return (
    <div
      className={`${color} wobbly p-4 shadow-hard rotate-[-1deg] hover:rotate-0 transition-transform`}
    >
      <p className="text-sm text-pencil/70">{label}</p>
      <p className="font-[family-name:var(--font-kalam)] text-2xl font-bold">{value}</p>
    </div>
  );
}
