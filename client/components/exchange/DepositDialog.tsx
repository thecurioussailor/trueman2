"use client";
import { useState } from "react";
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

type Props = { open: boolean; onOpenChange: (open: boolean) => void };
type Token = {
    id: string;
    symbol: "SOL" | "ETH" | "USDC" | "BTC";
    name: string;
    decimals: number;
    img: string;
  };

  const SUPPORTED_TOKENS: Token[] = [
    { id: "17d52355-12a8-4e4f-8876-85ca2d772a8a", symbol: "SOL",  name: "Solana",   decimals: 9, img: "https://backpack.exchange/_next/image?url=%2Fcoins%2Fsol.png&w=96&q=95" },
    { id: "650b575b-5928-4ec4-a5f9-2efcf3413f17", symbol: "ETH",  name: "Ethereum", decimals: 18, img: "https://backpack.exchange/_next/image?url=%2Fcoins%2Feth.png&w=96&q=95" },
    { id: "90207bf6-fcb9-48d2-8715-def20994b25f", symbol: "USDC", name: "USD Coin", decimals: 6, img: "https://backpack.exchange/_next/image?url=%2Fcoins%2Fusdt.png&w=96&q=95" },
    { id: "7b7ba2d4-e495-48c1-b903-7a45fc686ad8", symbol: "BTC",  name: "Bitcoin",  decimals: 8, img: "https://backpack.exchange/_next/image?url=%2Fcoins%2Fbtc.png&w=96&q=95" },
  ];
  
  // Allowed networks per token (adjust as you add support)
  const TOKEN_NETWORKS: Record<Token["symbol"], string[]> = {
    SOL: ["Solana"],
    ETH: ["Ethereum"],
    USDC: ["Solana"], // change later if you add more
    BTC: ["Bitcoin"],
  };

export default function DepositDialog({ open, onOpenChange }: Props) {
    const [asset, setAsset] = useState<Token>(SUPPORTED_TOKENS[0]);
    const [network, setNetwork] = useState<string>(TOKEN_NETWORKS[SUPPORTED_TOKENS[0].symbol][0]);
    const [amount, setAmount] = useState<string>("");
    const { deposit } = useBalances();
    const [submitting, setSubmitting] = useState(false);
  
  if (!open) return null;
  return (
    <div className="fixed h-screen inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/50" onClick={() => onOpenChange(false)} />
      <div className="relative z-10 w-full max-w-md rounded-xl border border-white/10 bg-[#0b0f14] p-5 text-zinc-100 shadow-2xl">
        <div className="mb-4 flex items-center justify-between">
            <h2 className="text-lg font-semibold">Deposit</h2>
        </div>
        {/* Asset */}
        <div className="mb-4">
          <label className="mb-2 block text-sm text-zinc-400">Asset</label>
          <Select value={asset.id} onValueChange={(value) => {
            const t = SUPPORTED_TOKENS.find((x) => x.id === value)!;
            setAsset(t);
          }}>
            <SelectTrigger className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 outline-none">
                <SelectValue placeholder="Select an asset" />
            </SelectTrigger>
            <SelectContent>
                <SelectGroup>
                {SUPPORTED_TOKENS.map((t) => (
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
          <Select value={network} onValueChange={(value) => setNetwork(value)}>
            <SelectTrigger className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 outline-none">
                <SelectValue placeholder="Select a network" />
            </SelectTrigger>
            <SelectContent>
                <SelectGroup>
                    {TOKEN_NETWORKS[asset.symbol].map((n) => (
                        <SelectItem key={n} value={n}>
                            {n}
                        </SelectItem>
                    ))}
                </SelectGroup>
            </SelectContent>
          </Select>
        </div>
        {/* Amount */}
        <div className="mb-4">
          <label className="mb-2 block text-sm text-zinc-400">Amount</label>
          <Input type="number" value={amount} min={0} onChange={(e: React.ChangeEvent<HTMLInputElement>) => setAmount(e.target.value)} className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 outline-none" />
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
                await deposit(asset.id, Number(amount));
                onOpenChange(false);
              } catch (error) {
                console.error(error);
              } finally {   
                setSubmitting(false);
              }
            }}
            className="h-9 rounded-lg bg-gradient-to-r from-violet-500 to-cyan-400 px-3 text-sm font-semibold text-black hover:brightness-110 disabled:opacity-50"
          >
            {submitting ? "Depositing..." : "Deposit"}
          </button>
        </div>
      </div>
    </div>
  );
}