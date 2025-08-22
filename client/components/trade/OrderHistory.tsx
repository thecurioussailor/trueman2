"use client";

import Link from "next/link";
import { useEffect, useMemo } from "react";
import { useSearchParams } from "next/navigation";
import { useOrders } from "@/store/order";

function canCancel(status?: string) {
  const s = (status || "").toUpperCase();
  return s === "PENDING" || s === "PARTIALLY_FILLED";
}

const OrderHistory = () => {
  const search = useSearchParams();
  const marketId = search.get("id") || undefined;

  const { items, loading, fetch, cancel } = useOrders();

  useEffect(() => {
    fetch(); // server returns all user orders; we'll filter client-side by marketId if present
  }, [fetch]);

  const rows = useMemo(
    () => (marketId ? items.filter((o) => o.market?.id === marketId) : items),
    [items, marketId]
  );

  return (
    <section className="rounded-xl border border-white/10 bg-white/5 p-3 lg:col-span-4 transition-all duration-300 ease-in-out">
      <div className="mb-2 flex items-center justify-between">
        <div className="text-sm font-semibold">Order History</div>
        <Link href="/user/orders" className="text-xs text-zinc-300 hover:text-white">
          View all
        </Link>
      </div>

      {/* Desktop table */}
      <div className="hidden md:block overflow-hidden rounded-lg border border-white/10">
        <table className="w-full border-collapse text-sm">
          <thead className="bg-white/5 text-zinc-300">
            <tr>
              <th className="px-3 py-2 text-left font-medium">Time</th>
              <th className="px-3 py-2 text-left font-medium">Market</th>
              <th className="px-3 py-2 text-left font-medium">Side</th>
              <th className="px-3 py-2 text-left font-medium">Kind</th>
              <th className="px-3 py-2 text-left font-medium">Price</th>
              <th className="px-3 py-2 text-left font-medium">Qty</th>
              <th className="px-3 py-2 text-left font-medium">Filled</th>
              <th className="px-3 py-2 text-left font-medium">Remaining</th>
              <th className="px-3 py-2 text-left font-medium">Status</th>
              <th className="px-3 py-2 text-right font-medium">Action</th>
            </tr>
          </thead>
          <tbody>
            {loading && (
              <tr>
                <td colSpan={10} className="px-3 py-6 text-center text-zinc-400">
                  Loading orders...
                </td>
              </tr>
            )}
            {!loading &&
              rows.map((o) => {
                const remaining = (o.quantity || 0) - (o.filled_quantity || 0);
                const cancellable = canCancel(o.status);
                return (
                  <tr key={o.id} className="border-t border-white/10">
                    <td className="px-3 py-2 text-zinc-300">
                      {new Date(o.created_at).toLocaleString()}
                    </td>
                    <td className="px-3 py-2">{o.market?.symbol?.replace("-", "/")}</td>
                    <td className={`px-3 py-2 ${o.order_type === "BUY" ? "text-emerald-400" : "text-rose-400"}`}>
                      {o.order_type}
                    </td>
                    <td className="px-3 py-2">{o.order_kind}</td>
                    <td className="px-3 py-2">{o.price ?? "-"}</td>
                    <td className="px-3 py-2">{o.quantity}</td>
                    <td className="px-3 py-2">{o.filled_quantity}</td>
                    <td className="px-3 py-2">{remaining}</td>
                    <td className="px-3 py-2">{o.status}</td>
                    <td className="px-3 py-2 text-right">
                      {cancellable ? (
                        <button
                          onClick={async () => {
                            await cancel({ order_id: o.id, market_id: o.market.id });
                          }}
                          className="rounded-lg border border-white/10 bg-white/5 px-3 py-1 text-xs font-semibold hover:bg-white/10"
                        >
                          Cancel
                        </button>
                      ) : (
                        <span className="text-xs text-zinc-400">—</span>
                      )}
                    </td>
                  </tr>
                );
              })}
            {!loading && rows.length === 0 && (
              <tr>
                <td colSpan={10} className="px-3 py-6 text-center text-zinc-400">
                  No orders
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {/* Mobile cards */}
      <div className="md:hidden space-y-2">
        {loading && (
          <div className="px-3 py-6 text-center text-zinc-400 border border-white/10 rounded-lg">
            Loading orders...
          </div>
        )}
        {!loading &&
          rows.map((o) => {
            const remaining = (o.quantity || 0) - (o.filled_quantity || 0);
            const cancellable = canCancel(o.status);
            return (
              <div key={o.id} className="rounded-lg border border-white/10 bg-black/20 p-3">
                <div className="flex justify-between text-xs text-zinc-400">
                  <span>{new Date(o.created_at).toLocaleString()}</span>
                  <span>{o.market?.symbol?.replace("-", "/")}</span>
                </div>
                <div className="mt-2 grid grid-cols-2 gap-2 text-sm">
                  <div>
                    <div className="text-zinc-400 text-xs">Side / Kind</div>
                    <div className={o.order_type === "Buy" ? "text-emerald-400" : "text-rose-400"}>
                      {o.order_type} • {o.order_kind}
                    </div>
                  </div>
                  <div>
                    <div className="text-zinc-400 text-xs">Price</div>
                    <div>{o.price ?? "-"}</div>
                  </div>
                  <div>
                    <div className="text-zinc-400 text-xs">Qty</div>
                    <div>{o.quantity}</div>
                  </div>
                  <div>
                    <div className="text-zinc-400 text-xs">Filled / Rem</div>
                    <div>
                      {o.filled_quantity} / {remaining}
                    </div>
                  </div>
                  <div className="col-span-2">
                    <div className="text-zinc-400 text-xs">Status</div>
                    <div>{o.status}</div>
                  </div>
                </div>
                <div className="mt-3 text-right">
                  {cancellable ? (
                    <button
                      onClick={async () => {
                        await cancel({ order_id: o.id, market_id: o.market.id });
                      }}
                      className="rounded-lg border border-white/10 bg-white/5 px-3 py-1 text-xs font-semibold hover:bg-white/10"
                    >
                      Cancel
                    </button>
                  ) : (
                    <span className="text-xs text-zinc-400">No actions</span>
                  )}
                </div>
              </div>
            );
          })}
        {!loading && rows.length === 0 && (
          <div className="px-3 py-6 text-center text-zinc-400 border border-white/10 rounded-lg">
            No orders
          </div>
        )}
      </div>
    </section>
  );
};

export default OrderHistory;