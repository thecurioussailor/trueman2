"use client";

import { notFound } from "next/navigation";
import Link from "next/link";
import { useMemo, useState } from "react";

type OBRow = { price: number; size: number; total: number };
type Trade = { time: string; price: number; size: number; side: "buy" | "sell" };

const asksMock: OBRow[] = [
  { price: 177.99, size: 279.6, total: 871.45 },
  { price: 177.98, size: 311.7, total: 591.85 },
  { price: 177.96, size: 37.21, total: 280.15 },
  { price: 177.95, size: 147.31, total: 242.94 },
  { price: 177.94, size: 35.4, total: 95.63 },
];
const bidsMock: OBRow[] = [
  { price: 177.89, size: 21.21, total: 21.21 },
  { price: 177.88, size: 88.09, total: 109.3 },
  { price: 177.85, size: 4.21, total: 113.51 },
  { price: 177.84, size: 270.67, total: 384.18 },
  { price: 177.83, size: 245.94, total: 630.12 },
];
const recentMock: Trade[] = [
  { time: "12:01:10", price: 177.90, size: 0.32, side: "sell" },
  { time: "12:01:06", price: 177.91, size: 13.63, side: "buy" },
  { time: "12:01:00", price: 177.92, size: 0.39, side: "buy" },
  { time: "12:00:58", price: 177.93, size: 45.89, side: "sell" },
  { time: "12:00:55", price: 177.94, size: 35.4, side: "sell" },
];

export default function TradePairPage({
  params,
}: {
  params: { pair: string }; // e.g. "sol-usdc"
}) {
  const slug = params.pair.toLowerCase();
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

      {/* Top nav */}
      <header className="sticky top-0 z-30 border-b border-white/10 bg-black/30 backdrop-blur">
        <div className="mx-auto flex h-14 max-w-7xl items-center justify-between px-4">
          <div className="flex items-center gap-3">
            <Link href="/" className="flex items-center gap-2">
              <Logo />
              <span className="hidden text-sm font-semibold tracking-tight sm:inline">Trueman</span>
            </Link>
            <span className="rounded-full border border-emerald-500/20 bg-emerald-500/10 px-2.5 py-0.5 text-xs font-semibold text-emerald-300">
              Spot
            </span>
          </div>
          <div className="hidden md:flex min-w-[440px] items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-3 py-1.5">
            <svg width="16" height="16" viewBox="0 0 24 24" className="text-zinc-400">
              <path fill="currentColor" d="M10 18a8 8 0 1 1 5.293-14.293L22 0l2 2l-6.707 6.707A8 8 0 0 1 10 18" />
            </svg>
            <input
              placeholder="Search markets"
              className="w-full bg-transparent text-sm outline-none placeholder:text-zinc-400"
            />
          </div>
          <div className="flex items-center gap-2">
            <Link
              href="/deposit"
              className="hidden h-9 rounded-lg border border-white/15 bg-white/5 px-3 text-sm font-medium hover:bg-white/10 md:inline-flex"
            >
              Deposit
            </Link>
            <Link
              href="/withdraw"
              className="hidden h-9 rounded-lg bg-gradient-to-r from-violet-500 to-cyan-400 px-3 text-sm font-semibold text-black hover:brightness-110 md:inline-flex"
            >
              Withdraw
            </Link>
            <div className="ml-1 inline-flex h-9 w-9 items-center justify-center rounded-full border border-white/10 bg-white/5 text-sm font-semibold">
              U
            </div>
          </div>
        </div>
      </header>

      {/* Market ticker strip */}
      <section className="border-b border-white/10 bg-black/20">
        <div className="mx-auto grid max-w-7xl grid-cols-1 gap-3 px-4 py-3 md:grid-cols-5">
          <div className="flex items-center gap-2 md:col-span-1">
            <div className="flex items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-2 py-1">
              <div className="h-6 w-6 rounded-full bg-gradient-to-br from-violet-500 to-cyan-400" />
              <span className="text-sm font-semibold">{symbol}</span>
            </div>
          </div>
          <Metric label="Price" value="177.89" />
          <Metric label="24H Change" value="+3.46%" pos />
          <Metric label="24H High / Low" value="179.65 / 171.67" />
          <Metric label="24H Volume" value="35,195,957" />
        </div>
      </section>

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
        <section className="rounded-xl border border-white/10 bg-white/5 p-3 lg:col-span-4">
          <div className="mb-2 flex items-center justify-between">
            <div className="text-sm font-semibold">Recent Trades</div>
            <Link href="/user/trades" className="text-xs text-zinc-300 hover:text-white">
              View all
            </Link>
          </div>
          <div className="overflow-hidden rounded-lg border border-white/10">
            <table className="w-full border-collapse text-sm">
              <thead className="bg-white/5 text-zinc-300">
                <tr>
                  <th className="px-3 py-2 text-left font-medium">Time</th>
                  <th className="px-3 py-2 text-left font-medium">Price</th>
                  <th className="px-3 py-2 text-left font-medium">Size</th>
                  <th className="px-3 py-2 text-left font-medium">Side</th>
                </tr>
              </thead>
              <tbody>
                {recentMock.map((t, i) => (
                  <tr key={i} className="border-t border-white/10">
                    <td className="px-3 py-2 text-zinc-300">{t.time}</td>
                    <td className={`px-3 py-2 ${t.side === "buy" ? "text-emerald-400" : "text-rose-400"}`}>
                      {t.price.toFixed(2)}
                    </td>
                    <td className="px-3 py-2">{t.size}</td>
                    <td className="px-3 py-2 capitalize">{t.side}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </main>
  );
}

function Metric({ label, value, pos }: { label: string; value: string; pos?: boolean }) {
  return (
    <div className="flex items-center gap-3 rounded-lg border border-white/10 bg-white/5 px-3 py-2">
      <div className="text-xs text-zinc-400">{label}</div>
      <div className={`text-sm font-bold ${pos ? "text-emerald-400" : ""}`}>{value}</div>
    </div>
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

function BookTrades() {
  const [tab, setTab] = useState<"book" | "trades">("book");
  const mid = 177.89;

  return (
    <div>
      <div className="mb-2 flex items-center gap-2">
        <button
          onClick={() => setTab("book")}
          className={`rounded-lg px-3 py-1.5 text-sm ${tab === "book" ? "bg-white/10 border border-white/15" : "text-zinc-300 hover:bg-white/5"}`}
        >
          Book
        </button>
        <button
          onClick={() => setTab("trades")}
          className={`rounded-lg px-3 py-1.5 text-sm ${tab === "trades" ? "bg-white/10 border border-white/15" : "text-zinc-300 hover:bg-white/5"}`}
        >
          Trades
        </button>
      </div>

      {tab === "book" ? (
        <div className="grid grid-cols-1 gap-2">
          <div className="overflow-hidden rounded-lg border border-white/10">
            <table className="w-full border-collapse text-xs">
              <thead className="bg-white/5 text-zinc-300">
                <tr>
                  <th className="px-2 py-1.5 text-left font-medium">Price (USD)</th>
                  <th className="px-2 py-1.5 text-left font-medium">Size</th>
                  <th className="px-2 py-1.5 text-left font-medium">Total</th>
                </tr>
              </thead>
              <tbody>
                {asksMock.map((r, i) => (
                  <tr key={`a-${i}`} className="border-t border-white/10">
                    <td className="px-2 py-1.5 text-rose-400">{r.price.toFixed(2)}</td>
                    <td className="px-2 py-1.5">{r.size}</td>
                    <td className="px-2 py-1.5">{r.total}</td>
                  </tr>
                ))}
                <tr className="bg-white/5">
                  <td className="px-2 py-1.5 font-bold text-emerald-300">{mid.toFixed(2)}</td>
                  <td className="px-2 py-1.5 text-zinc-400">—</td>
                  <td className="px-2 py-1.5 text-zinc-400">—</td>
                </tr>
                {bidsMock.map((r, i) => (
                  <tr key={`b-${i}`} className="border-t border-white/10">
                    <td className="px-2 py-1.5 text-emerald-400">{r.price.toFixed(2)}</td>
                    <td className="px-2 py-1.5">{r.size}</td>
                    <td className="px-2 py-1.5">{r.total}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ) : (
        <div className="overflow-hidden rounded-lg border border-white/10">
          <table className="w-full border-collapse text-xs">
            <thead className="bg-white/5 text-zinc-300">
              <tr>
                <th className="px-2 py-1.5 text-left font-medium">Time</th>
                <th className="px-2 py-1.5 text-left font-medium">Price</th>
                <th className="px-2 py-1.5 text-left font-medium">Size</th>
              </tr>
            </thead>
            <tbody>
              {recentMock.map((t, i) => (
                <tr key={i} className="border-t border-white/10">
                  <td className="px-2 py-1.5">{t.time}</td>
                  <td className={`px-2 py-1.5 ${t.side === "buy" ? "text-emerald-400" : "text-rose-400"}`}>
                    {t.price.toFixed(2)}
                  </td>
                  <td className="px-2 py-1.5">{t.size}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function OrderForm({ base, quote, midPrice }: { base: string; quote: string; midPrice: number }) {
  const [side, setSide] = useState<"buy" | "sell">("buy");
  const [kind, setKind] = useState<"Limit" | "Market" | "Conditional">("Limit");
  const [price, setPrice] = useState(midPrice.toFixed(2));
  const [qty, setQty] = useState("");
  const [postOnly, setPostOnly] = useState(false);
  const [ioc, setIoc] = useState(false);
  const [margin, setMargin] = useState(false);

  const btnClass =
    side === "buy"
      ? "bg-emerald-600 hover:bg-emerald-500"
      : "bg-rose-600 hover:bg-rose-500";

  const sideTitle = side === "buy" ? "Buy" : "Sell";

  return (
    <div>
      <div className="mb-3 flex items-center gap-2">
        <button
          onClick={() => setSide("buy")}
          className={`h-9 flex-1 rounded-lg text-sm font-semibold ${
            side === "buy" ? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/20" : "bg-white/5 text-zinc-200"
          }`}
        >
          Buy
        </button>
        <button
          onClick={() => setSide("sell")}
          className={`h-9 flex-1 rounded-lg text-sm font-semibold ${
            side === "sell" ? "bg-rose-500/20 text-rose-300 border border-rose-500/20" : "bg-white/5 text-zinc-200"
          }`}
        >
          Sell
        </button>
      </div>

      <div className="mb-3 flex items-center gap-2">
        {(["Limit", "Market", "Conditional"] as const).map((k) => (
          <button
            key={k}
            onClick={() => setKind(k)}
            className={`rounded-lg px-3 py-1.5 text-xs ${
              kind === k ? "bg-white/10 border border-white/15" : "text-zinc-300 hover:bg-white/5"
            }`}
          >
            {k}
          </button>
        ))}
      </div>

      <div className="space-y-3">
        {kind !== "Market" && (
          <LabeledInput
            label="Price"
            suffix={quote}
            value={price}
            onChange={setPrice}
            placeholder={midPrice.toFixed(2)}
          />
        )}
        <LabeledInput
          label="Quantity"
          suffix={base}
          value={qty}
          onChange={setQty}
          placeholder="0"
        />

        <div className="rounded-lg border border-white/10 bg-black/20 p-3">
          <div className="text-xs text-zinc-400">Order Value</div>
          <div className="mt-1 text-lg font-bold">
            {qty && (parseFloat(qty) * parseFloat(kind === "Market" ? `${midPrice}` : price || "0")).toFixed(2)} {quote}
          </div>
        </div>

        <button className={`mt-1 w-full rounded-lg px-3 py-2 font-semibold text-black ${btnClass}`}>
          {sideTitle}
        </button>

        <div className="mt-2 grid grid-cols-3 gap-2 text-xs text-zinc-300">
          <label className="flex items-center gap-2">
            <input type="checkbox" checked={postOnly} onChange={(e) => setPostOnly(e.target.checked)} className="accent-emerald-500" />
            Post Only
          </label>
          <label className="flex items-center gap-2">
            <input type="checkbox" checked={ioc} onChange={(e) => setIoc(e.target.checked)} className="accent-emerald-500" />
            IOC
          </label>
          <label className="flex items-center gap-2">
            <input type="checkbox" checked={margin} onChange={(e) => setMargin(e.target.checked)} className="accent-emerald-500" />
            Margin
          </label>
        </div>
      </div>
    </div>
  );
}

function LabeledInput({
  label,
  suffix,
  value,
  onChange,
  placeholder,
}: {
  label: string;
  suffix: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
}) {
  return (
    <label className="block">
      <div className="mb-1 text-xs text-zinc-300">{label}</div>
      <div className="flex items-center rounded-lg border border-white/10 bg-black/30 px-2">
        <input
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className="w-full bg-transparent px-1 py-2 text-sm outline-none"
        />
        <span className="ml-2 rounded-md border border-white/10 bg-white/5 px-2 py-1 text-xs text-zinc-300">
          {suffix}
        </span>
      </div>
    </label>
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