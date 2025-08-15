import Link from "next/link";
import { useEffect } from "react";
import { useMarkets } from "@/store/markets";
  
const Markets = () => {
  const { markets, loading, error, fetchPublic } = useMarkets();

  useEffect(() => {
    fetchPublic();
  }, [fetchPublic]);

  return (
    <div className="mx-auto max-w-7xl px-4 py-6">

        {/* Markets table */}
        <div className="overflow-hidden rounded-xl border border-white/10 bg-black/5">
          <table className="w-full border-collapse text-sm">
            <thead className="bg-black/30 text-zinc-300">
              <tr>
                <th className="px-6 py-6 text-left font-bold text-zinc-300">Market</th>
                <th className="px-6 py-6 text-left font-bold text-zinc-300">Price</th>
                <th className="px-6 py-6 text-left font-bold text-zinc-300">24h Change</th>
                <th className="px-6 py-6 text-left font-bold text-zinc-300">24h Volume</th>
                <th className="px-6 py-6 text-right font-bold">Trade</th>
              </tr>
            </thead>
            <tbody>
            {loading && (
              <tr>
                <td className="px-4 py-8 text-center text-zinc-400" colSpan={5}>
                  Loading markets...
                </td>
              </tr>
            )}
            {!loading &&
              markets.map((m) => {
                // Public markets API does not include price/change/volume; placeholders for now
                const symbolDisplay = m.symbol.replace("-", "/");
                return (
                  <tr key={m.id} className="border-t border-white/10">
                    <td className="px-4 py-3 font-semibold">{symbolDisplay}</td>
                    <td className="px-4 py-3">-</td>
                    <td className="px-4 py-3">-</td>
                    <td className="px-4 py-3">-</td>
                    <td className="px-4 py-3 text-right">
                      <Link
                        href={{
                          pathname: `/trade/${m.symbol}`,
                          query: { id: m.id },
                        }}
                        className="inline-flex h-8 items-center rounded-lg border border-white/10 bg-white/5 px-3 text-xs font-semibold hover:bg-white/10"
                      >
                        Open
                      </Link>
                    </td>
                  </tr>
                );
              })}
            {!loading && markets.length === 0 && (
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