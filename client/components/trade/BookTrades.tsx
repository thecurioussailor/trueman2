import { useState } from "react";

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

export default function BookTrades() {
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
  