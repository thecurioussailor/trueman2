// client/store/marketFeed.ts
import { create } from 'zustand';
import { wsClient } from '@/lib/wsClient';

type Depth = { bids: [number, number][]; asks: [number, number][]; seq?: number };
type Ticker = { last_price: number; volume_24h: number; high_24h: number; low_24h: number; change_24h: number; timestamp: number };
type Trade = { id: string; price: number; quantity: number; buyer_user_id?: string; seller_user_id?: string; timestamp: number };

type MarketSlice = {
  currentMarketId?: string;
  depthByMarket: Record<string, Depth>;
  tickerByMarket: Record<string, Ticker>;
  tradesByMarket: Record<string, Trade[]>;
  subscribeMarket: (marketId: string) => void;
  clearMarket: (marketId: string) => void;
};

export const useMarketFeedStore = create<MarketSlice>((set, get) => {
  // Wire WS → store once
  if (typeof window !== 'undefined') {
    wsClient.connect();
    wsClient.addListener((msg: any) => {
      if (msg?.type !== 'event' || !msg.channel) return;
      const ch: string = msg.channel;
      const payload = msg.payload;

      if (ch.startsWith('depth:')) {
        const marketId = ch.split(':')[1];
        set(state => ({
          depthByMarket: { ...state.depthByMarket, [marketId]: { bids: payload.bids ?? [], asks: payload.asks ?? [], seq: payload.seq } }
        }));
      } else if (ch.startsWith('ticker:')) {
        const marketId = ch.split(':')[1];
        set(state => ({
          tickerByMarket: { ...state.tickerByMarket, [marketId]: payload }
        }));
      } else if (ch.startsWith('trades:')) {
        const marketId = ch.split(':')[1];
        set(state => ({
          tradesByMarket: {
            ...state.tradesByMarket,
            [marketId]: [payload, ...(state.tradesByMarket[marketId] ?? [])].slice(0, 200)
          }
        }));
      }
    });
  }

  return {
    currentMarketId: undefined,
    depthByMarket: {},
    tickerByMarket: {},
    tradesByMarket: {},

    subscribeMarket: (marketId: string) => {
      set({ currentMarketId: marketId });
      wsClient.subscribe(marketId, ['depth', 'ticker', 'trades']);
    },

    clearMarket: (marketId: string) => {
      set(state => ({
        depthByMarket: { ...state.depthByMarket, [marketId]: { bids: [], asks: [] } },
        tickerByMarket: { ...state.tickerByMarket, [marketId]: undefined as any },
        tradesByMarket: { ...state.tradesByMarket, [marketId]: [] }
      }));
    },
  };
});