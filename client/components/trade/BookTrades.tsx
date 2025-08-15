// client/components/trade/BookTrades.tsx
'use client';

import { useEffect, useMemo, useRef, useState } from 'react';
import { useMarketFeedStore } from '@/store/marketFeed';
import { ScrollArea } from "@/components/ui/scroll-area"
import { Button } from '../ui/button';
import { fromAtomic } from '@/lib/units';
type OBRow = { price: number; size: number };
type RowView = OBRow & { total: number; pct: number; bg: string };

const EMPTY_TRADES: any[] = []; // stable reference
const EMPTY_LEVELS: [number, number][] = [];

function buildDepth(
  side: 'asks' | 'bids',
  rows: OBRow[],
  opts?: { useNotional?: boolean; anchor?: 'left' | 'right'; reverseCum?: boolean }
): RowView[] {
  const useNotional = opts?.useNotional ?? false;
  const anchor = opts?.anchor ?? 'right';
  const reverseCum = opts?.reverseCum ?? false;

  const src = reverseCum ? [...rows].reverse() : rows;
  let cum = 0;
  const withCum = src.map((r) => {
    const val = useNotional ? r.price * r.size : r.size;
    cum += val;
    return { ...r, total: cum };
  });
  const max = withCum.at(-1)?.total || 1;
  const normalized = withCum.map((r) => {
    const pct = Math.max(0, Math.min(100, (r.total / max) * 100));
    const color = side === 'asks' ? 'rgba(244,63,94,0.16)' : 'rgba(34,197,94,0.16)';
    const dir = anchor === 'right' ? 'to left' : 'to right';
    const bg = `linear-gradient(${dir}, ${color} ${pct}%, transparent ${pct}%)`;
    return { ...r, pct, bg };
  });
  return reverseCum ? normalized.reverse() : normalized;
}

export default function BookTrades() {
  const [tab, setTab] = useState<'book' | 'trades'>('book');
  
  const marketId = useMarketFeedStore((s) => s.currentMarketId);
  
  const depth = useMarketFeedStore((s) => (marketId ? s.depthByMarket[marketId] : undefined));
  const tradesSel = useMarketFeedStore((s) => (marketId ? s.tradesByMarket[marketId] : undefined));

  const asksRows = useMemo(
    () => (depth?.asks ?? EMPTY_LEVELS).map(([p, q]) => ({ price: Number(p), size: q })),
    [depth]
  );
  const bidsRows = useMemo(
    () => (depth?.bids ?? EMPTY_LEVELS).map(([p, q]) => ({ price: Number(p), size: q })),
    [depth]
  );

  // Best levels for mid; asks are ascending; bids are descending in your feed
  const bestAsk = asksRows[0]?.price;
  const bestBid = bidsRows[0]?.price;
  const mid = bestAsk && bestBid ? (bestAsk + bestBid) / 2 : undefined;

  const asksDepth = useMemo(
    () => buildDepth('asks', [...asksRows].reverse(), { reverseCum: true, anchor: 'right' }),
    [asksRows]
  );
  const bidsDepth = useMemo(
    () => buildDepth('bids', bidsRows, { reverseCum: false, anchor: 'right' }),
    [bidsRows]
  );

  const trades = tradesSel ?? EMPTY_TRADES; // stable fallback
  const recentTrades = useMemo(() => {
    const list = trades.slice(0, 200);
    return list.map((t: any) => {
      const time = new Date(t.timestamp).toLocaleTimeString();
      const side = mid !== undefined && t.price >= mid ? 'buy' as const : 'sell' as const;
      return { time, price: t.price, size: t.quantity, side };
    });
  }, [trades, mid]);

  const bodyRef = useRef<HTMLTableSectionElement | null>(null);
  const midRowRef = useRef<HTMLTableRowElement | null>(null);
  const hasUserScrolledRef = useRef(false);

  const scrollMidIntoCenter = (smooth = false) => {
    const body = bodyRef.current;
    const midEl = midRowRef.current;
    if (!body || !midEl) return;
    const bodyRect = body.getBoundingClientRect();
    const midRect = midEl.getBoundingClientRect();
    const delta = midRect.top - bodyRect.top;
    const target = body.scrollTop + delta - (body.clientHeight / 2 - midRect.height / 2);
    const max = body.scrollHeight - body.clientHeight;
    const top = Math.max(0, Math.min(max, target));
    if (smooth) {
      body.scrollTo({ top, behavior: 'smooth' });
    } else {
      body.scrollTop = top;
    }
  }
  useEffect(() => {
    const el = bodyRef.current;
    if (!el) return;
    const onScroll = () => { hasUserScrolledRef.current = true; };
    el.addEventListener('scroll', onScroll, { passive: true });
    return () => el.removeEventListener('scroll', onScroll);
  }, []);

  useEffect(() => {
    if (tab !== 'book') return;
    if (hasUserScrolledRef.current) return;
    scrollMidIntoCenter(true);
  }, [tab, mid, asksDepth.length, bidsDepth.length]);

  return (
    <div>
      <div className="mb-2 flex items-center gap-2">
        <button
          onClick={() => setTab('book')}
          className={`rounded-lg px-3 py-1.5 text-sm ${
            tab === 'book' ? 'bg-white/10 border border-white/15' : 'text-zinc-300 hover:bg-white/5'
          }`}
        >
          Book
        </button>
        <button
          onClick={() => setTab('trades')}
          className={`rounded-lg px-3 py-1.5 text-sm ${
            tab === 'trades' ? 'bg-white/10 border border-white/15' : 'text-zinc-300 hover:bg-white/5'
          }`}
        >
          Trades
        </button>
      </div>

      {tab === 'book' ? (
        <div className="grid grid-cols-1 gap-2">
          <div className="relative overflow-hidden rounded-lg border border-white/10">
            <div className="absolute z-10 top-0 left-0 w-full bg-gray-900 text-zinc-300 text-xs">
                <div className="flex items-center justify-between w-full ">
                  <div className="px-2 py-1.5 text-left font-medium">Price</div>
                  <div className="px-2 py-1.5 text-left font-medium">Size</div>
                  <div className="px-2 py-1.5 text-left font-medium">Total</div>
                </div>
              </div>
          <div
          ref={bodyRef}
                className="block h-120 overflow-y-auto [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
          > 
          <div className="relative w-full  text-xs border-collapse">
              <div className="pt-7">
                  {(asksDepth.length ? asksDepth : []).map((r, i) => (
                    <div key={`a-${i}`} className="flex justify-between border-t border-white/10" style={{ backgroundImage: r.bg, backgroundRepeat: 'no-repeat' }}>
                      <div className="px-2 py-1.5 text-rose-400">{r.price.toFixed(2)}</div>
                      <div className="px-2 py-1.5">{r.size}</div>
                      <div className="px-2 py-1.5">{r.total}</div>
                    </div>
                  ))}
                  <div ref={midRowRef} className="flex items-center justify-between bg-white/5">
                    <div className="px-2 py-1.5 font-bold text-emerald-300">{mid !== undefined ? mid.toFixed(2) : '—'}</div>
                    <div className="px-2 py-1.5 text-zinc-400"></div>
                    <div className="px-2 py-1.5">
                      {hasUserScrolledRef.current && (
                        <Button 
                        variant="ghost"
                        size="sm"
                        className="text-blue-500 hover:text-blue-600 cursor-pointer"
                        onClick={() => {
                          hasUserScrolledRef.current = false;
                          scrollMidIntoCenter(true);
                        }}>
                          Recentre
                        </Button>
                      )}
                    </div>
                  </div>
                  {(bidsDepth.length ? bidsDepth : []).map((r, i) => (
                    <div key={`b-${i}`} className="flex justify-between border-t border-white/10" style={{ backgroundImage: r.bg, backgroundRepeat: 'no-repeat' }}>
                      <div className="px-2 py-1.5 text-emerald-400">{r.price.toFixed(2)}</div>
                      <div className="px-2 py-1.5">{r.size}</div>
                      <div className="px-2 py-1.5">{r.total}</div>
                    </div>
                  ))}
              </div>
              </div>
              </div>
          </div>
        </div>
      ) : (
        <div className="overflow-hidden rounded-lg border border-white/10">
          <ScrollArea className='h-120'>
            <table className="w-full border-collapse text-xs">
              <thead className="bg-white/5 text-zinc-300">
                <tr>
                  <th className="px-2 py-1.5 text-left font-medium">Time</th>
                  <th className="px-2 py-1.5 text-left font-medium">Price</th>
                  <th className="px-2 py-1.5 text-left font-medium">Size</th>
                </tr>
              </thead>
              <tbody>
                {recentTrades.map((t, i) => (
                  <tr key={i} className="border-t border-white/10">
                    <td className="px-2 py-1.5">{t.time}</td>
                    <td className={`px-2 py-1.5 ${t.side === 'buy' ? 'text-emerald-400' : 'text-rose-400'}`}>{t.price}</td>
                    <td className="px-2 py-1.5">{t.size}</td>
                  </tr>
                ))}
                {!recentTrades.length && (
                  <tr><td className="px-2 py-2 text-zinc-400" colSpan={3}>No trades yet</td></tr>
                )}
              </tbody>
            </table>
          </ScrollArea>
        </div>
      )}
    </div>
  );
}