"use client";

import { notFound, useParams } from "next/navigation";
import { useState } from "react";
import Ticker from "@/components/trade/Ticker";
import RecentTrades from "@/components/trade/RecentTrades";
import OrderForm from "@/components/trade/OrderForm";
import BookTrades from "@/components/trade/BookTrades";

export default function TradePairPage() {
    const { pair } = useParams();
    const slug = pair?.toString().toLowerCase() || "";
    const parts = slug.split("-");
    if (parts.length !== 2) return notFound();

    const base = parts[0].toUpperCase();
    const quote = parts[1].toUpperCase();
    const symbol = `${base}/${quote}`;

  return (
    <main className="min-h-screen bg-[#0b0f14] text-zinc-100">
      {/* BG */}
      <div className="pointer-events-none fixed inset-0 -z-10">
        <div className="absolute -top-40 -left-32 h-[42rem] w-[42rem] rounded-full blur-3xl bg-gradient-to-br from-violet-600/30 to-cyan-400/20" />
        <div className="absolute -top-32 right-0 h-[36rem] w-[36rem] rounded-full blur-3xl bg-gradient-to-tr from-cyan-400/20 to-violet-600/20" />
      </div>
      <Ticker symbol={symbol} />

      {/* Main grid: 4 columns on lg - chart(2) / book(1) / order(1) */}
      <div className="mx-auto grid max-w-7xl grid-cols-1 gap-4 px-4 py-4 lg:grid-cols-4">
        {/* Chart + tabs */}
        <section className="rounded-xl border border-white/10 bg-white/5 p-3 lg:col-span-2">
          <Tabs tabs={["Chart", "Depth", "Margin"]} />
          <div className="mt-3 h-[420px] rounded-lg border border-white/10 bg-black/20" />
        </section>

        {/* Book / Trades */}
        <section className="rounded-xl border border-white/10 bg-white/5 p-3">
          <BookTrades />
        </section>

        {/* Order form */}
        <section className="rounded-xl border border-white/10 bg-white/5 p-3">
          <OrderForm base={base} quote={quote} midPrice={177.78} />
        </section>

        {/* Bottom: Recent trades spanning all */}
        <RecentTrades />
      </div>
    </main>
  );
}

function Tabs({ tabs }: { tabs: string[] }) {
  const [active, setActive] = useState(0);
  return (
    <div className="flex gap-2">
      {tabs.map((t, i) => (
        <button
          key={t}
          onClick={() => setActive(i)}
          className={`rounded-lg px-3 py-1.5 text-sm ${
            active === i ? "bg-white/10 border border-white/15" : "text-zinc-300 hover:bg-white/5"
          }`}
        >
          {t}
        </button>
      ))}
    </div>
  );
}

function Logo() {
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" aria-hidden className="block">
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