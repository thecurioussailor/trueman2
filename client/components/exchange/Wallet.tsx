import { useBalances } from "@/store/balances";
import { useTokens } from "@/store/tokens";
import { useEffect, useMemo } from "react";

const Wallet = () => {
    const { items, fetch, loading, error } = useBalances();
    const { tokens, fetchPublic } = useTokens();

    useEffect(() => {
        fetch();
        if (!tokens.length) fetchPublic();
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    const rows = useMemo(() => {
        return (items ?? [])
            .map((b) => {
                const t = tokens.find((x) => x.id === b.token_id);
                const symbol = t?.symbol ?? b.token_id.slice(0, 4).toUpperCase();
                const img = t?.symbol
                    ? `https://backpack.exchange/_next/image?url=%2Fcoins%2F${t.symbol.toLowerCase()}.png&w=96&q=95`
                    : undefined;
                return { id: b.token_id, symbol, img, available: b.available };
            })
            .sort((a, b) => b.available - a.available);
    }, [items, tokens]);

    return (
    <div className="w-[280px]">
        <div className="overflow-hidden">
            <div className="flex items-center justify-between px-2">
                <h3 className="text-sm text-zinc-400">Wallet</h3>
            </div>
            <div className="max-h-80 overflow-y-auto">
                {loading ? (
                    <div className="p-4 space-y-3">
                        <div className="h-6 w-full rounded bg-white/5" />
                        <div className="h-6 w-full rounded bg-white/5" />
                        <div className="h-6 w-full rounded bg-white/5" />
                    </div>
                ) : rows.length > 0 ? (
                    rows.map((r) => (
                        <div
                            key={r.id}
                            className="px-2 py-3 flex items-center justify-between transition-colors"
                        >
                            <div className="flex items-center gap-3">
                                {r.img ? (
                                    <img src={r.img} alt={r.symbol} className="h-6 w-6 rounded-full" />
                                ) : (
                                    <div className="h-6 w-6 rounded-full bg-white/10 grid place-items-center text-xs">
                                        {r.symbol[0]}
                                    </div>
                                )}
                                <div className="flex flex-col">
                                    <span className="text-sm font-medium">{r.symbol}</span>
                                    <span className="text-xs text-zinc-400">Available</span>
                                </div>
                            </div>
                            <div className="text-sm font-semibold tabular-nums">
                                {Number(r.available).toLocaleString(undefined, { maximumFractionDigits: 8 })}
                            </div>
                        </div>
                    ))
                ) : (
                    <div className="px-4 py-6 text-sm text-zinc-400">No balances found</div>
                )}
            </div>
        </div>
        {error ? <div className="mt-2 text-xs text-red-400">{error}</div> : null}
    </div>
    );
}

export default Wallet