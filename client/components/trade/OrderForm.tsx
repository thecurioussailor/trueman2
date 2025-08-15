"use client";
import { useState } from "react";
import { useOrders } from "@/store/order";
import { useMarketFeedStore } from "@/store/marketFeed";
export default function OrderForm({ base, quote, marketId }: { base: string; quote: string; marketId: string }) {
    
    const [side, setSide] = useState<"buy" | "sell">("buy");
    const [kind, setKind] = useState<"Limit" | "Market">("Limit");
    const [price, setPrice] = useState("");
    const [qty, setQty] = useState("");

    const { create, loading, error } = useOrders();
    const ticker = useMarketFeedStore(s => s.tickerByMarket[marketId]);
    const submit = async () => {
      if (!marketId || !qty) return;
      const order_type = side === "buy" ? "Buy" : "Sell";
      const order_kind = kind === "Market" ? "Market" : "Limit";
      const p = order_kind === "Market" ? null : Number(price || 0);
      const q = Number(qty);
      // TODO: convert p and q to engine integer units if needed (based on market tick_size/min_order_size)
      await create({ market_id: marketId, order_type, order_kind, price: p, quantity: q });
      setQty("");
    };

    const btnClass =
      side === "buy"
        ? "bg-emerald-600 hover:bg-emerald-500"
        : "bg-rose-600 hover:bg-rose-500";
  
    const sideTitle = side === "buy" ? "Buy" : "Sell";
  
    return (
      <div>
        <div className="mb-3 flex items-center gap-2">
          <button
            onClick={() => setSide("buy")}
            className={`h-9 flex-1 rounded-lg text-sm font-semibold ${
              side === "buy" ? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/20" : "bg-white/5 text-zinc-200"
            }`}
          >
            Buy
          </button>
          <button
            onClick={() => setSide("sell")}
            className={`h-9 flex-1 rounded-lg text-sm font-semibold ${
              side === "sell" ? "bg-rose-500/20 text-rose-300 border border-rose-500/20" : "bg-white/5 text-zinc-200"
            }`}
          >
            Sell
          </button>
        </div>
  
        <div className="mb-3 flex items-center gap-2">
          {(["Limit", "Market"] as const).map((k) => (
            <button
              key={k}
              onClick={() => setKind(k)}
              className={`rounded-lg px-3 py-1.5 text-xs ${
                kind === k ? "bg-white/10 border border-white/15" : "text-zinc-300 hover:bg-white/5"
              }`}
            >
              {k}
            </button>
          ))}
        </div>
  
        <div className="space-y-3">
          {kind !== "Market" && (
            <LabeledInput
              label="Price"
              suffix={quote}
              value={price}
              onChange={setPrice}
              placeholder={ticker?.last_price.toFixed(2)}
            />
          )}
          <LabeledInput
            label="Quantity"
            suffix={base}
            value={qty}
            onChange={setQty}
            placeholder="0"
          />
  
          <div className="rounded-lg border border-white/10 bg-black/20 p-3">
            <div className="text-xs text-zinc-400">Order Value</div>
            <div className="mt-1 text-lg font-bold">
              {qty && (parseFloat(qty) * parseFloat(kind === "Market" ? `${ticker?.last_price}` : price || "0")).toFixed(2)} {quote}
            </div>
          </div>
  
          <button 
            className={`mt-1 w-full rounded-lg px-3 py-2 font-semibold text-black ${btnClass}`}
          onClick={submit}
          disabled={loading}
          >
            {loading ? "Placing..." : sideTitle}
          </button>
        </div>
      </div>
    );
  }

  function LabeledInput({
    label,
    suffix,
    value,
    onChange,
    placeholder,
  }: {
    label: string;
    suffix: string;
    value: string;
    onChange: (v: string) => void;
    placeholder?: string;
  }) {
    return (
      <label className="block">
        <div className="mb-1 text-xs text-zinc-300">{label}</div>
        <div className="flex items-center rounded-lg border border-white/10 bg-black/30 px-2">
          <input
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder={placeholder}
            className="w-full bg-transparent px-1 py-2 text-sm outline-none"
          />
          <span className="ml-2 rounded-md border border-white/10 bg-white/5 px-2 py-1 text-xs text-zinc-300">
            {suffix}
          </span>
        </div>
      </label>
    );
  }
