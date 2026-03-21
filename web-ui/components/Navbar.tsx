"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { StampIcon, ShieldCheckIcon, SettingsIcon, ListIcon } from "./Icons";

const links = [
  { href: "/", label: "Sign", icon: StampIcon },
  { href: "/verify", label: "Verify", icon: ShieldCheckIcon },
  { href: "/admin", label: "Admin", icon: SettingsIcon },
  { href: "/events", label: "Events", icon: ListIcon },
];

export default function Navbar() {
  const pathname = usePathname();

  return (
    <nav className="mb-8">
      <div className="max-w-4xl mx-auto px-4 py-4 flex items-center justify-between">
        <Link href="/" className="font-[family-name:var(--font-kalam)] text-2xl font-bold text-pen">
          Threshold Timestamp Server
        </Link>
        <div className="flex gap-6">
          {links.map(({ href, label, icon: Icon }) => {
            const active = pathname === href;
            return (
              <Link
                key={href}
                href={href}
                className={`flex items-center gap-1.5 text-lg transition-colors ${
                  active
                    ? "wavy-underline text-pen font-bold"
                    : "text-pencil hover:text-pen"
                }`}
              >
                <Icon className="w-5 h-5" />
                {label}
              </Link>
            );
          })}
        </div>
      </div>
    </nav>
  );
}
