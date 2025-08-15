"use client";

import { useEffect, useState } from "react";
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectTrigger,
    SelectValue,
  } from "@/components/ui/select"
import { Input } from "@/components/ui/input"
import { useBalances } from "@/store/balances";
import { useTokens } from "@/store/tokens";
import { ChevronDown, ChevronUp } from "lucide-react";
type Token = {
    id: string;
    symbol: "SOL" | "ETH" | "USDC" | "BTC";
    name: string;
    decimals: number;
    img: string;
    is_active: boolean,
    created_at: string
  };

  // Allowed networks per token (adjust as you add support)
  const TOKEN_NETWORKS: Record<Token["symbol"], string[]> = {
    SOL: ["Solana"],
    ETH: ["Ethereum"],
    USDC: ["Solana"],
    BTC: ["Bitcoin"],
  };

type Props = { open: boolean; onOpenChange: (open: boolean) => void };

export default function WithdrawDialog({ open, onOpenChange }: Props) {
    const [filteredTokens, setFilteredTokens] = useState<Token[]>([])

    const [asset, setAsset] = useState<Token | null>(null);
    const [network, setNetwork] = useState<string | null>(null);
    const [amount, setAmount] = useState<string>("");
    const { withdraw, fetch: loadBalances, items, loading: loadingBalances } = useBalances();
    const { tokens, fetchPublic: loadTokens, loading: loadingTokens } = useTokens();
    const [submitting, setSubmitting] = useState(false);

    useEffect(() => {
        if (open) {
            loadBalances();
            loadTokens();
        }
    }, [open, loadBalances, loadTokens]);

    // derive filtered tokens when tokens change
  useEffect(() => {
    if (!open || !tokens?.length) return;
    const mapped = tokens.map((t) => ({
      ...t,
      img: `https://backpack.exchange/_next/image?url=%2Fcoins%2F${t.symbol.toLowerCase()}.png&w=96&q=95`,
    }));
    setFilteredTokens(mapped as Token[]);
  }, [open, tokens]);

  if (!open) return null;


  return (
    <div className="fixed h-screen inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/50" onClick={() => onOpenChange(false)} />
      <div className="relative z-10 w-full max-w-md rounded-xl border border-white/10 bg-[#0b0f14] p-5 text-zinc-100 shadow-2xl">
        <h2 className="mb-3 text-lg font-semibold">Withdraw</h2>
        {/* Amount */}
        <div className="mb-4">
          <div className="flex flex-row items-start justify-between gap-2">
            <label className="mb-2 block text-sm text-zinc-400">Amount</label>
            <span className="text-sm text-[#4c94ff] font-semibold">
                Transferable: {asset && items.find((x) => x.token_id === asset.id)?.available || 0} {asset?.symbol}
            </span>
          </div>
          <div className="relative">
            <Input 
              type="number" 
              value={amount} min={0} 
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setAmount(e.target.value)} 
              className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 outline-none" />
            <div className="pointer-events-auto absolute inset-y-1 right-1 flex w-10 flex-col justify-between">
              <button
                type="button"
                aria-label="Increase amount"
                className="grid h-5 place-items-center text-zinc-600 cursor-pointer"
                onClick={() => {
                  const v = parseFloat(amount || "0");
                  const next = Number.isFinite(v) ? v + 1 : 1; // step = 1; adjust as needed
                  setAmount(String(next));
                }}
              >
                <ChevronUp size={14} />
              </button>
              <button
                type="button"
                aria-label="Decrease amount"
                className="grid h-5 place-items-center text-zinc-600 cursor-pointer"
                onClick={() => {
                  const v = parseFloat(amount || "0");
                  const next = Math.max(0, Number.isFinite(v) ? v - 1 : 0);
                  setAmount(String(next));
                }}
                disabled={parseFloat(amount || "0") <= 0}
              >
                <ChevronDown size={14} />
              </button>
            </div>
          </div>
        </div>
        {/* Asset */}
        <div className="mb-4">
          <label className="mb-2 block text-sm text-zinc-400">Asset</label>
          <Select value={asset?.id} onValueChange={(value) => {
            const t = filteredTokens.find((x) => x.id === value)!;
            setAsset(t);
          }}>
            <SelectTrigger className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 outline-none">
                <SelectValue placeholder="Select an asset" />
            </SelectTrigger>
            <SelectContent>
                <SelectGroup>
                {filteredTokens.map((t) => (
                    <SelectItem key={t.id} value={t.id}>
                        <div className="flex items-center gap-4">
                            <img src={t.img} alt={t.symbol} className="w-5 h-5" />
                            {t.symbol}
                        </div>
                    </SelectItem>
                ))}
                </SelectGroup>
            </SelectContent>
        </Select>
        </div>
        {/* Network */}
        <div className="mb-4">
          <label className="mb-2 block text-sm text-zinc-400">Network</label>
          <Select value={network || ""} onValueChange={(value) => setNetwork(value)}>
            <SelectTrigger className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 outline-none">
                <SelectValue placeholder="Select a network" />
            </SelectTrigger>
            <SelectContent>
                <SelectGroup>
                    {TOKEN_NETWORKS[asset?.symbol as keyof typeof TOKEN_NETWORKS]?.map((n) => (
                        <SelectItem key={n} value={n}>
                            {n}
                        </SelectItem>
                    ))}
                </SelectGroup>
            </SelectContent>
          </Select>
        </div>
        {/* Actions */}
        <div className="mt-5 flex justify-end gap-2">
          <button
            onClick={() => onOpenChange(false)}
            className="h-9 rounded-lg border border-white/15 bg-white/5 px-3 text-sm"
          >
            Cancel
          </button>
          <button
            disabled={submitting}
            onClick={async () => {
              setSubmitting(true);
              try {
                await withdraw(asset?.id || "", Number(amount));
                await loadBalances();
                onOpenChange(false);
              } catch (error) {
                console.error(error);
              } finally {   
                setSubmitting(false);
              }
            }}
            className="h-9 rounded-lg bg-gradient-to-r from-violet-500 to-cyan-400 px-3 text-sm font-semibold text-black hover:brightness-110 disabled:opacity-50"
          >
            {submitting ? "Withdrawing..." : "Withdraw"}
          </button>
        </div>
      </div>
    </div>
  );
}