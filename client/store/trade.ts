import { create } from "zustand";
import { api } from "@/lib/http";

export type EngineTrade = {
  id: string;
  market_id: string;
  buyer_order_id: string;
  seller_order_id: string;
  buyer_user_id: string;
  seller_user_id: string;
  price: number;      // i64 on backend
  quantity: number;   // i64 on backend
  created_at: string; // ISO string
};

type FetchParams = {
  market_id?: string;
  limit?: number;
  offset?: number;
};

type State = {
  items: EngineTrade[];
  loading: boolean;
  error: string | null;
  fetch: (p?: FetchParams) => Promise<void>;
  reset: () => void;
};

export const useTrades = create<State>((set) => ({
  items: [],
  loading: false,
  error: null,

  fetch: async (p = {}) => {
    set({ loading: true, error: null });
    try {
      const { data } = await api.get<EngineTrade[]>("/user/trades", {
        params: {
          market_id: p.market_id,
          limit: p.limit,
          offset: p.offset,
        },
      });
      set({ items: Array.isArray(data) ? data : [] });
    } catch (e: any) {
      set({ error: e?.message || "Failed to load trades" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  reset: () => set({ items: [], error: null }),
}));