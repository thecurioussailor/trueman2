import Link from "next/link";

const RecentTrades = () => {
    const recentMock = [
        { time: "2021-01-01 12:00:00", price: 100, size: 100, side: "buy" },
        { time: "2021-01-01 12:00:00", price: 100, size: 100, side: "sell" },
        { time: "2021-01-01 12:00:00", price: 100, size: 100, side: "buy" },
    ];
  return (
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
  )
}

export default RecentTrades