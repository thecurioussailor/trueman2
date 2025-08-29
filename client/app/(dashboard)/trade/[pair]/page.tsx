"use client";
import { notFound, useParams, useSearchParams } from "next/navigation";
import { useState, useEffect, useMemo } from "react";
import { useMarketFeedStore } from "@/store/marketFeed";
import Ticker from "@/components/trade/Ticker";
import RecentTrades from "@/components/trade/RecentTrades";
import OrderForm from "@/components/trade/OrderForm";
import BookTrades from "@/components/trade/BookTrades";
import OrderHistory from "@/components/trade/OrderHistory";
import { useSimulator } from "@/store/simulator";
import TradingViewChart from "@/components/trade/TradingViewChart";
import { toAtomic } from "@/lib/units";
import { IoIosArrowUp } from "react-icons/io";
import { IoInformation } from "react-icons/io5";
import { Play } from "lucide-react";
import { FaXTwitter } from "react-icons/fa6";
import Link from "next/link";
import { useMarkets } from "@/store/markets";


// helpers (top of file)
function mapPair(pair?: string) {
  console.log("pair", pair);
  const p = (pair || "btc-usdc").toUpperCase().replace(/[^A-Z]/g, "");
  console.log("p", p);
  if (p.endsWith("USDC")) return { ex: "BINANCE" as const, sym: p, tv: `BINANCE:${p}` };
  // USD default → Coinbase
  const coin = p.replace("USDC", "");
  return { ex: "COINBASE" as const, sym: `${coin}-USDT`, tv: `COINBASE:${coin}USDT` };
}

async function getMidClient(ex: "BINANCE" | "COINBASE", sym: string): Promise<number> {
  if (ex === "BINANCE") {
    const r = await fetch(`https://api.binance.com/api/v3/ticker/bookTicker?symbol=${sym}`, { cache: "no-store" });
    const j = await r.json();
    return (parseFloat(j.bidPrice) + parseFloat(j.askPrice)) / 2;
  } else {
    // Coinbase Exchange ticker; may fail due to CORS in some regions
    const r = await fetch(`https://api.exchange.coinbase.com/products/${sym}/ticker`, { cache: "no-store" });
    const j = await r.json();
    return (parseFloat(j.bid) + parseFloat(j.ask)) / 2;
  }
}

async function fetchStartMidTruncated(pair: string, fallback = 120): Promise<number> {
  try {
    const { ex, sym } = mapPair(pair);
    const mid = await getMidClient(ex, sym);
    return Number.isFinite(mid) ? Math.trunc(mid) : fallback;
  } catch {
    return fallback;
  }
}

export default function TradePairPage() {
    const { pair } = useParams();
    const search = useSearchParams();
    const marketId = useMemo(() => search.get("id"), [search]); 
    const { markets, fetchPublic } = useMarkets(); 

    const slug = pair?.toString().toLowerCase() || "";
    const parts = slug.split("-");
    if (parts.length !== 2) return notFound();

    const base = parts[0].toUpperCase();
    const quote = parts[1].toUpperCase();
    const symbol = `${base}/${quote}`;
    const symbolTradingView = `${base.toUpperCase()}${quote.toUpperCase()}`;

    // Find the current market data
    const currentMarket = useMemo(() => {
      return markets.find(market => market.id === marketId);
  }, [markets, marketId]);

    const [bottomTab, setBottomTab] = useState<"recent" | "history">("recent");

    const { start, loading } = useSimulator();
    const [fabOpen, setFabOpen] = useState(false);
    const [showTooltip, setShowTooltip] = useState(false);
    async function handleStartSimulator() {
      if (!currentMarket) {
        console.log("Market data not available");
        return;
      } 
      const start_mid = await fetchStartMidTruncated(pair as string);
      await start({
        market_id: marketId || "",
        base_token_id: currentMarket.base_currency.id,
        quote_token_id: currentMarket.quote_currency.id,
        users: 8,
        base_deposit: 0.1,
        quote_deposit: 100,
        order_rate_ms: 100,
        min_qty: 0.001,
        max_qty: 0.01,
        start_mid,
        tick: 0.01,
      });
      setFabOpen(false);
    }

    // Load markets when component mounts
    useEffect(() => {
      fetchPublic();
  }, [fetchPublic]);

    useEffect(() => {
      if (!marketId) return;
      useMarketFeedStore.getState().subscribeMarket(marketId);
    }, [marketId]);

    // useEffect(() => {
    //   if (!marketId || !currentMarket) return;
    //   (async () => {
    //     const start_mid = await fetchStartMidTruncated(pair as string);
    //     useSimulator.getState().start({
    //     market_id: marketId,
    //     base_token_id: currentMarket.base_currency.id,
    //     quote_token_id: currentMarket.quote_currency.id,
    //     users: 8,
    //     base_deposit: 0.1,
    //     quote_deposit: 100,
    //     order_rate_ms: 100,
    //     min_qty: 0.001,
    //     max_qty: 0.01,
    //     start_mid: start_mid,
    //     tick: 0.01,
    //   });
    // })();
    // }, [marketId, currentMarket, pair]);

// Add loading state while market data is being fetched
if (marketId && !currentMarket) {
  return (
      <main className="min-h-screen bg-[#0b0f14] text-zinc-100 flex items-center justify-center">
          <div className="text-center">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-zinc-100 mx-auto mb-4"></div>
              <p>Loading market data...</p>
          </div>
      </main>
  );
}

  return (
    <main className="min-h-screen bg-[#0b0f14] text-zinc-100">
      {/* BG */}
      <div className="pointer-events-none fixed inset-0 -z-10">
        <div className="absolute -top-40 -left-32 h-[42rem] w-[42rem] rounded-full blur-3xl bg-gradient-to-br from-violet-600/30 to-cyan-400/20" />
        <div className="absolute -top-32 right-0 h-[36rem] w-[36rem] rounded-full blur-3xl bg-gradient-to-tr from-cyan-400/20 to-violet-600/20" />
      </div>
      <div className="fixed bottom-5 right-5 z-50">
        <div className="flex flex-col items-end gap-2">
          {fabOpen && (
            <div className="mb-2 w-52 overflow-hidden rounded-xl border border-white/10 bg-black/60 p-2 shadow-xl backdrop-blur">
              <button
                onClick={handleStartSimulator}
                disabled={loading}
                className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-zinc-100 hover:bg-white/10 disabled:opacity-60"
              >
                <Play size={16} />
                <span>{loading ? "Starting…" : "Start simulator"}</span>
              </button>
              <Link
                href="https://x.com/sagar11ashutosh"
                target="_blank"
                rel="noreferrer"
                className="mt-1 flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-zinc-100 hover:bg-white/10"
              >
                <FaXTwitter size={16} />
                <span>Follow me</span>
              </Link>
              <div className="pointer-events-none mt-2 select-none text-center text-xs text-zinc-400">
                Made with <span className="text-rose-400">❤</span>
              </div>
            </div>
          )}

          <button
            onClick={() => setFabOpen((v) => !v)}
            className="cursor-pointer grid h-12 w-12 place-items-center rounded-full border border-white/10 bg-white/10 text-zinc-100 shadow-lg backdrop-blur transition hover:bg-white/15"
            aria-label="Quick actions"
          >
            <IoIosArrowUp className={`transition-transform ${fabOpen ? "rotate-180" : ""}`} size={22} />
          </button>
        </div>
      </div>
      <Ticker symbol={symbol} marketId={marketId || ""} />
      {/* Main grid: 4 columns on lg - chart(2) / book(1) / order(1) */}
      <div className="mx-auto grid grid-cols-1 gap-4 px-4 py-4 lg:grid-cols-4">
        {/* Chart + tabs */}
        <section className="lg:col-span-2">
          <div className="flex flex-col gap-2">
            <div className="text-sm justify-between text-zinc-400 flex items-center gap-2">
              <span className="text-zinc-400">You can start simulator from bottom right corner.</span>
              <div 
                onMouseEnter={() => setShowTooltip(true)}
                onMouseLeave={() => setShowTooltip(false)}
                className="relative bg-white/10 rounded-full p-1 cursor-pointer">
                {showTooltip && (
                  <div className="absolute -top-16 left-0 w-48 bg-zinc-900 rounded-lg p-2 z-50">
                    <span className="text-zinc-400">Orderbook data may differ from tradingview chart data.</span>
                  </div>
                )}
                <IoInformation size={16} />
              </div>
            </div>
            <div className="overflow-hidden rounded-lg border border-white/10 bg-black/20">
              <TradingViewChart symbol={`BINANCE:${symbolTradingView}`} height={420} />
            </div>
          </div>
        </section>
        {/* Book / Trades */}
        <section className="rounded-xl border border-white/10 bg-white/5 p-3">
          <BookTrades />
        </section>

        {/* Order form */}
        <section className="rounded-xl border border-white/10 bg-white/5 p-3">
          <OrderForm base={base} quote={quote} marketId={marketId || ""} />
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