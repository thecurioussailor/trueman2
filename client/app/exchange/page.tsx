"use client";

import Link from "next/link";
import { useMemo, useState } from "react";

type Market = {
  symbol: string;    // e.g. "SOL/USDC"
  price: string;     // e.g. "168.24"
  change24h: string; // "+2.1%" or "-0.9%"
  volume24h: string; // e.g. "2.3M SOL"
};

const MOCK_MARKETS: Market[] = [
  { symbol: "BTC/USDC", price: "68420.12", change24h: "+2.4%", volume24h: "12,345 BTC" },
  { symbol: "ETH/USDC", price: "3420.08", change24h: "-1.2%", volume24h: "98,210 ETH" },
  { symbol: "SOL/USDC", price: "168.02", change24h: "+5.1%", volume24h: "2,340,100 SOL" },
];

export default function ExchangePage() {
  const [q, setQ] = useState("");

  const filtered = useMemo(() => {
    const s = q.trim().toLowerCase();
    if (!s) return MOCK_MARKETS;
    return MOCK_MARKETS.filter((m) => m.symbol.toLowerCase().includes(s));
  }, [q]);

  return (
    <main className="min-h-screen bg-[#0b0f14] text-zinc-100">
      {/* BG gradients */}
      <div className="pointer-events-none fixed inset-0 -z-10">
        <div className="absolute -top-40 -left-32 h-[42rem] w-[42rem] rounded-full blur-3xl bg-gradient-to-br from-violet-600/30 to-cyan-400/20" />
        <div className="absolute -top-32 right-0 h-[36rem] w-[36rem] rounded-full blur-3xl bg-gradient-to-tr from-cyan-400/20 to-violet-600/20" />
      </div>

      {/* Navbar */}
      <header className="sticky top-0 z-20 border-b border-white/10 bg-black/20 backdrop-blur">
        <div className="mx-auto flex h-16 max-w-7xl items-center gap-3 px-4">
          {/* Left: Logo + name */}
          <Link href="/" className="flex items-center gap-2">
            <Logo />
            <span className="hidden text-sm font-semibold tracking-tight sm:inline">Trueman</span>
          </Link>

          {/* Middle: Search + Spot pill */}
          <div className="mx-3 flex flex-1 items-center gap-3">
            <div className="hidden items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-3 py-2 md:flex md:min-w-[340px]">
              <svg width="16" height="16" viewBox="0 0 24 24" className="text-zinc-400">
                <path fill="currentColor" d="M10 18a8 8 0 1 1 5.293-14.293L22 0l2 2l-6.707 6.707A8 8 0 0 1 10 18m0-2a6 6 0 1 0 0-12a6 6 0 0 0 0 12" />
              </svg>
              <input
                value={q}
                onChange={(e) => setQ(e.target.value)}
                placeholder="Search markets (e.g. BTC, SOL)"
                className="w-full bg-transparent text-sm outline-none placeholder:text-zinc-400"
              />
            </div>
            <span className="hidden rounded-full border border-emerald-500/20 bg-emerald-500/10 px-2.5 py-1 text-xs font-semibold text-emerald-300 md:inline">
              Spot
            </span>
          </div>

          {/* Right: Actions */}
          <div className="flex items-center gap-2">
            <Link
              href="/deposit"
              className="md:flex justify-center items-center hidden h-9 rounded-lg border border-white/15 bg-white/5 px-3 text-sm font-medium hover:bg-white/10"
            >
              Deposit
            </Link>
            <Link
              href="/withdraw"
              className="md:flex justify-center items-center hidden h-9 rounded-lg bg-gradient-to-r from-violet-500 to-cyan-400 px-3 text-sm font-semibold text-black hover:brightness-110"
            >
              Withdraw
            </Link>

            {/* Avatar (placeholder) */}
            <div className="ml-1 inline-flex h-9 w-9 items-center justify-center rounded-full border border-white/10 bg-white/5 text-sm font-semibold">
              U
            </div>
          </div>
        </div>
      </header>

      {/* Content */}
      <div className="mx-auto max-w-7xl px-4 py-6">
        {/* Mobile search */}
        <div className="mb-4 md:hidden">
          <div className="flex items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-3 py-2">
            <svg width="16" height="16" viewBox="0 0 24 24" className="text-zinc-400">
              <path fill="currentColor" d="M10 18a8 8 0 1 1 5.293-14.293L22 0l2 2l-6.707 6.707A8 8 0 0 1 10 18m0-2a6 6 0 1 0 0-12a6 6 0 0 0 0 12" />
            </svg>
            <input
              value={q}
              onChange={(e) => setQ(e.target.value)}
              placeholder="Search markets"
              className="w-full bg-transparent text-sm outline-none placeholder:text-zinc-400"
            />
          </div>
        </div>

        {/* Markets table */}
        <div className="overflow-hidden rounded-xl border border-white/10 bg-white/5">
          <table className="w-full border-collapse text-sm">
            <thead className="bg-white/5 text-zinc-300">
              <tr>
                <th className="px-4 py-3 text-left font-medium">Market</th>
                <th className="px-4 py-3 text-left font-medium">Price</th>
                <th className="px-4 py-3 text-left font-medium">24h Change</th>
                <th className="px-4 py-3 text-left font-medium">24h Volume</th>
                <th className="px-4 py-3 text-right font-medium">Trade</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((m) => {
                const pos = m.change24h.startsWith("+");
                return (
                  <tr key={m.symbol} className="border-t border-white/10">
                    <td className="px-4 py-3 font-semibold">{m.symbol}</td>
                    <td className="px-4 py-3">${m.price}</td>
                    <td className={`px-4 py-3 font-semibold ${pos ? "text-emerald-400" : "text-rose-400"}`}>
                      {m.change24h}
                    </td>
                    <td className="px-4 py-3">{m.volume24h}</td>
                    <td className="px-4 py-3 text-right">
                      <Link
                        href={`/trade/${m.symbol.replace("/", "-")}`}
                        className="inline-flex h-8 items-center rounded-lg border border-white/10 bg-white/5 px-3 text-xs font-semibold hover:bg-white/10"
                      >
                        Open
                      </Link>
                    </td>
                  </tr>
                );
              })}
              {filtered.length === 0 && (
                <tr>
                  <td className="px-4 py-8 text-center text-zinc-400" colSpan={5}>
                    No markets found
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        {/* Hints/links row */}
        <div className="mt-4 flex flex-wrap items-center justify-between gap-3">
          <div className="text-xs text-zinc-400">Showing Spot markets only.</div>
          <div className="flex gap-3 text-xs">
            <Link className="text-zinc-300 hover:text-white" href="/user/orders">
              My Orders
            </Link>
            <Link className="text-zinc-300 hover:text-white" href="/user/trades">
              My Trades
            </Link>
            <Link className="text-zinc-300 hover:text-white" href="/balances">
              Balances
            </Link>
          </div>
        </div>
      </div>
    </main>
  );
}

function Logo() {
  return (
    <svg width="26" height="26" viewBox="0 0 24 24" aria-hidden className="block">
      <defs>
        <linearGradient id="gx" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#7C3AED" />
          <stop offset="100%" stopColor="#06B6D4" />
        </linearGradient>
      </defs>
      <path fill="url(#gx)" d="M12 2l9 5v10l-9 5-9-5V7l9-5zm0 2.2L5 7v8l7 3.8L19 15V7l-7-2.8z" />
    </svg>
  );
}