
const Ticker = ({ symbol }: { symbol: string }) => {
  return (
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