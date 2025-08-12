"use client";

import Link from "next/link";
import { useEffect } from "react";
import { useSearchParams } from "next/navigation";
import { useTrades } from "@/store/trade";
import { useAuth } from "@/store/auth";

const RecentTrades = () => {
  const search = useSearchParams();
  const marketId = search.get("id") || "";
  const { items, loading, fetch } = useTrades();
  const { id: userId } = useAuth();

  useEffect(() => {
    fetch({ market_id: marketId, limit: 50, offset: 0 });
    console.log(marketId);
  }, [fetch, marketId]);

  return (
    <section className="rounded-xl border border-white/10 bg-white/5 p-3 lg:col-span-4 transition-all duration-300 ease-in-out">
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
            {loading && (
              <tr>
                <td colSpan={4} className="px-3 py-6 text-center text-zinc-400">
                  Loading trades...
                </td>
              </tr>
            )}
            {!loading &&
              items.map((t) => (
                <tr key={t.id} className="border-t border-white/10">
                  <td className="px-3 py-2 text-zinc-300">{new Date(t.created_at).toLocaleString()}</td>
                  <td className="px-3 py-2">{t.price}</td>
                  <td className="px-3 py-2">{t.quantity}</td>
                  <td className="px-3 py-2 capitalize">{t.buyer_user_id === userId ? <span className="text-green-500">Buy</span> : t.seller_user_id === userId ? <span className="text-red-500">Sell</span> : "-"}</td>
                </tr>
              ))}   
            {!loading && items.length === 0 && (
              <tr>
                <td colSpan={4} className="px-3 py-6 text-center text-zinc-400">
                  No recent trades
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
};

export default RecentTrades;