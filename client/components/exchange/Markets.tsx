import Link from "next/link";

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
const Markets = () => {
  return (
    <div className="mx-auto max-w-7xl px-4 py-6">

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
              {MOCK_MARKETS.map((m) => {
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
              {MOCK_MARKETS.length === 0 && (
                <tr>
                  <td className="px-4 py-8 text-center text-zinc-400" colSpan={5}>
                    No markets found
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
    </div>
  )
}

export default Markets