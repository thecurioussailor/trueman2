import { useMarketFeedStore } from "@/store/marketFeed";

const Ticker = ({ symbol, marketId }: { symbol: string, marketId: string }) => {
  const ticker = useMarketFeedStore(s => s.tickerByMarket[marketId]);
  return (
    <section className="border-b border-white/10 bg-black/20">
    <div className="mx-auto grid max-w-7xl grid-cols-1 gap-3 px-4 py-3 md:grid-cols-5">
      <div className="flex items-center gap-2 md:col-span-1">
        <div className="flex items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-2 py-1">
          <div className="h-6 w-6 rounded-full bg-gradient-to-br from-violet-500 to-cyan-400" />
          <span className="text-sm font-semibold">{symbol}</span>
        </div>
      </div>
      <Metric label="Price" value={ticker?.last_price.toFixed(2)} />
      <Metric label="24H Change" value={`${ticker?.change_24h.toFixed(2)}%`} pos={ticker?.change_24h > 0} />
      <Metric label="24H High / Low" value={`${ticker?.high_24h.toFixed(2)} / ${ticker?.low_24h.toFixed(2)}`} />
      <Metric label="24H Volume" value={ticker?.volume_24h.toLocaleString()} />
    </div>
  </section>
  )
}

export default Ticker

function Metric({ label, value, pos }: { label: string; value: string; pos?: boolean }) {
    return (
      <div className="flex items-center gap-3 rounded-lg border border-white/10 bg-white/5 px-3 py-2">
        <div className="text-xs text-zinc-400">{label}</div>
        <div className={`text-sm font-bold ${pos ? "text-emerald-400" : ""}`}>{value}</div>
      </div>
    );
  }