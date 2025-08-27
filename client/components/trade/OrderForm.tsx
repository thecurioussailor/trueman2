"use client";
import { useEffect, useMemo, useState } from "react";
import { useOrders } from "@/store/order";
import { useMarketFeedStore } from "@/store/marketFeed";
import { useMarkets } from "@/store/markets";
export default function OrderForm({ base, quote, marketId }: { base: string; quote: string; marketId: string }) {
    
    const [side, setSide] = useState<"buy" | "sell">("buy");
    const [kind, setKind] = useState<"Limit" | "Market">("Limit");
    const [price, setPrice] = useState("");
    const [qty, setQty] = useState("");
    const [priceInitialized, setPriceInitialized] = useState(false);

    const { create, loading, error } = useOrders();
    const { markets } = useMarkets();
    const ticker = useMarketFeedStore(s => s.tickerByMarket[marketId]);

    // Find the current market
    const market = useMemo(() => 
      markets.find(m => m.id === marketId), 
      [markets, marketId]
    );

    // Initialize price with ticker price only once
    useEffect(() => {
      if (!market) return;
      if (ticker?.last_price && !priceInitialized && !price) {
           // Calculate decimal places needed for tick size
          const tickSizeDecimal = market.tick_size / Math.pow(10, market.quote_currency.decimals);
          const decimalPlaces = Math.max(0, -Math.floor(Math.log10(tickSizeDecimal)));
          setPrice(ticker.last_price.toFixed(decimalPlaces));
          setQty(0.0.toFixed(decimalPlaces));
          setPriceInitialized(true);
      }
    }, [ticker?.last_price, priceInitialized, price]);

    // Reset initialization when market changes
    useEffect(() => {
      setPriceInitialized(false);
      setPrice("");
    }, [marketId]);

    // Calculate decimal constraints
    const constraints = useMemo(() => {
      if (!market) return null;

      const minOrderSizeDecimal = market.min_order_size / Math.pow(10, market.base_currency.decimals);
      const tickSizeDecimal = market.tick_size / Math.pow(10, market.quote_currency.decimals);

      return {
          minOrderSize: minOrderSizeDecimal,
          tickSize: tickSizeDecimal,
          baseDecimals: market.base_currency.decimals,
          quoteDecimals: market.quote_currency.decimals,
      };
    }, [market]);

    // Validation function
    const validateOrder = () => {
      if (!constraints) return { valid: false, errors: ["Market not found"] };
      
      const errors: string[] = [];
      const qtyNum = parseFloat(qty);
      const priceNum = parseFloat(price);

      // Validate quantity
      if (isNaN(qtyNum) || qtyNum <= 0) {
          errors.push("Quantity must be greater than 0");
      } else if (qtyNum < constraints.minOrderSize) {
          errors.push(`Minimum order size is ${constraints.minOrderSize} ${base}`);
      }

      // Validate price for limit orders
      if (kind === "Limit") {
          if (isNaN(priceNum) || priceNum <= 0) {
              errors.push("Price must be greater than 0");
          } else {
              // Check tick size compliance
              const remainder = (priceNum * Math.pow(10, constraints.quoteDecimals)) % market!.tick_size;
              if (remainder !== 0) {
                  errors.push(`Price must be in increments of ${constraints.tickSize} ${quote}`);
              }
          }
      }

      return { valid: errors.length === 0, errors };
  };

  const validation = useMemo(() => validateOrder(), [qty, price, kind, constraints, base, quote, market]);


    const submit = async () => {
      if (!marketId || !qty) return;

      const order_type = side === "buy" ? "Buy" : "Sell";
      const order_kind = kind === "Market" ? "Market" : "Limit";
      const p = order_kind === "Market" ? null : Number(price || 0);
      const q = Number(qty);
      // TODO: convert p and q to engine integer units if needed (based on market tick_size/min_order_size)
      await create({ market_id: marketId, order_type, order_kind, price: p, quantity: q });
      setQty("");
      setPrice("");
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
              placeholder={ticker?.last_price.toFixed(constraints?.quoteDecimals || 2)}
              step={constraints?.tickSize || 0.1}
            />
          )}
          <LabeledInput
            label="Quantity"
            suffix={base}
            value={qty}
            onChange={setQty}
            placeholder="0"
            min={constraints?.minOrderSize}
          />
          {/* Validation Errors */}
          {!validation.valid && validation.errors.length > 0 && (
              <div className="rounded-lg border border-red-500/20 bg-red-500/10 p-3">
                  <div className="text-xs text-red-400">Validation Errors:</div>
                  <ul className="mt-1 text-xs text-red-300">
                      {validation.errors.map((error, idx) => (
                          <li key={idx}>• {error}</li>
                      ))}
                  </ul>
              </div>
          )}
          {/* Market Constraints Info */}
          {constraints && (
              <div className="rounded-lg border border-white/10 bg-black/20 p-3">
                  <div className="text-xs text-zinc-400">Market Info</div>
                  <div className="mt-1 space-y-1 text-xs text-zinc-300">
                      <div>Min Order: {constraints.minOrderSize} {base}</div>
                      <div>Tick Size: {constraints.tickSize} {quote}</div>
                  </div>
              </div>
          )}
  
          <div className="rounded-lg border border-white/10 bg-black/20 p-3">
            <div className="text-xs text-zinc-400">Order Value</div>
            <div className="mt-1 text-lg font-bold">
              {qty && (parseFloat(qty) * parseFloat(kind === "Market" ? `${ticker?.last_price}` : price || "0")).toFixed(2)} {quote}
            </div>
          </div>
  
          <button 
                    className={`mt-1 w-full rounded-lg px-3 py-2 font-semibold text-black ${btnClass} ${
                        !validation.valid ? 'opacity-50 cursor-not-allowed' : ''
                    }`}
                    onClick={submit}
                    disabled={loading || !validation.valid}
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
    step,
    min,
  }: {
    label: string;
    suffix: string;
    value: string;
    onChange: (v: string) => void;
    placeholder?: string;
    step?: number;
    min?: number;
  }) {
    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      let inputValue = e.target.value;
      
      // Allow empty string
      if (inputValue === '') {
          onChange('');
          return;
      }
      
      // Remove any non-digit and non-decimal characters
      inputValue = inputValue.replace(/[^0-9.]/g, '');
      
      // Ensure only one decimal point
      const parts = inputValue.split('.');
      
      if (parts.length > 1) {
          inputValue = parts[0] + '.' + parts.slice(1).join('');
      }
      
      // Limit to 2 decimal places
      const decimalPlaces = Math.max(0, -Math.floor(Math.log10(step || 0.1)));
      if (parts.length === 2 && parts[1].length > decimalPlaces) {
          inputValue = parts[0] + '.' + parts[1].substring(0, decimalPlaces);
      }
      
      onChange(inputValue);
  };
    return (
      <label className="block">
        <div className="mb-1 text-xs text-zinc-300">{label}</div>
        <div className="flex items-center rounded-lg border border-white/10 bg-black/30 px-2">
          <input
            value={value}
            onChange={handleChange}
            placeholder={placeholder}
            className="w-full bg-transparent px-1 py-2 text-sm outline-none"
            step={step}
            min={min}
          />
          <span className="ml-2 rounded-md border border-white/10 bg-white/5 px-2 py-1 text-xs text-zinc-300">
            {suffix}
          </span>
        </div>
      </label>
    );
  }
