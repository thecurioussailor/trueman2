"use client";
import { notFound, useParams, useSearchParams } from "next/navigation";
import { useState, useEffect, useMemo } from "react";
import { useMarketFeedStore } from "@/store/marketFeed";
import Ticker from "@/components/trade/Ticker";
import RecentTrades from "@/components/trade/RecentTrades";
import OrderForm from "@/components/trade/OrderForm";
import BookTrades from "@/components/trade/BookTrades";
import OrderHistory from "@/components/trade/OrderHistory";

export default function TradePairPage() {
    const { pair } = useParams();
    const search = useSearchParams();
    const marketId = useMemo(() => search.get("id"), [search]); 

    const slug = pair?.toString().toLowerCase() || "";
    const parts = slug.split("-");
    if (parts.length !== 2) return notFound();

    const base = parts[0].toUpperCase();
    const quote = parts[1].toUpperCase();
    const symbol = `${base}/${quote}`;

    const [bottomTab, setBottomTab] = useState<"recent" | "history">("recent");

    useEffect(() => {
      if (!marketId) return;
      useMarketFeedStore.getState().subscribeMarket(marketId);
    }, [marketId]);
  

  return (
    <main className="min-h-screen bg-[#0b0f14] text-zinc-100">
      {/* BG */}
      <div className="pointer-events-none fixed inset-0 -z-10">
        <div className="absolute -top-40 -left-32 h-[42rem] w-[42rem] rounded-full blur-3xl bg-gradient-to-br from-violet-600/30 to-cyan-400/20" />
        <div className="absolute -top-32 right-0 h-[36rem] w-[36rem] rounded-full blur-3xl bg-gradient-to-tr from-cyan-400/20 to-violet-600/20" />
      </div>
      <Ticker symbol={symbol} marketId={marketId || ""} />
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
          <OrderForm base={base} quote={quote} midPrice={177.78} marketId={marketId || ""} />
        </section>

        {/* Bottom: Recent trades spanning all */}
        <div className="lg:col-span-4">
          <div className="mb-2 flex items-center gap-2">
            <button
              onClick={() => setBottomTab("recent")}
              className={`rounded-lg px-3 py-1.5 text-sm ${
                bottomTab === "recent"
                  ? "bg-white/10 border border-white/15"
                  : "text-zinc-300 hover:bg-white/5"
              }`}
            >
              Recent Trades
            </button>
            <button
              onClick={() => setBottomTab("history")}
              className={`rounded-lg px-3 py-1.5 text-sm ${
                bottomTab === "history"
                  ? "bg-white/10 border border-white/15"
                  : "text-zinc-300 hover:bg-white/5"
              }`}
            >
              Order History
            </button>
          </div>
          {bottomTab === "recent" ? <RecentTrades /> : <OrderHistory />}
        </div>
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