import { create } from "zustand";
import { api } from "@/lib/http";

export type EngineOrder = {
  id: string;
  user_id: string;
  market: Market;
  order_type: string;   // "Buy" | "Sell"
  order_kind: string;   // "Market" | "Limit"
  price: number | null; // backend Option<i64>
  quantity: number;
  filled_quantity: number;
  status: string;
  created_at: string;
  updated_at: string;
};

export type Market = {
    id: string;
    symbol: string;
    base_currency_id: string;
    quote_currency_id: string;
    min_order_size: number;
    tick_size: number;
    is_active: boolean;
    created_at: string;
}

export type OrderEngineTradeInfo = {
  trade_id: string;
  price: number;
  quantity: number;
  timestamp: number;
};

export type OrderEngineResponse = {
  request_id: string;
  success: boolean;
  status: string; // "FILLED" | "PARTIALLY_FILLED" | "PENDING" | "REJECTED"
  order_id?: string | null;
  message: string;
  filled_quantity?: number | null;
  remaining_quantity?: number | null;
  average_price?: number | null;
  trades?: OrderEngineTradeInfo[] | null;
};

export type CreateOrderPayload = {
  market_id: string;
  order_type: "Buy" | "Sell";
  order_kind: "Market" | "Limit";
  price?: number | null;
  quantity: number;
};

type State = {
  items: EngineOrder[];
  loading: boolean;
  error: string | null;
  fetch: () => Promise<void>;
  create: (p: CreateOrderPayload) => Promise<OrderEngineResponse>;
  cancel: (p: { order_id: string; market_id: string }) => Promise<OrderEngineResponse>;
};

export const useOrders = create<State>((set, get) => ({
  items: [],
  loading: false,
  error: null,

  fetch: async () => {
    set({ loading: true, error: null });
    try {
      const { data } = await api.get<EngineOrder[]>("/user/orders");
      set({ items: Array.isArray(data) ? data : [] });
    } catch (e: any) {
      set({ error: e?.message || "Failed to load orders" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  create: async (p) => {
    set({ loading: true, error: null });
    try {
      const { data } = await api.post<OrderEngineResponse>("/user/orders", {
        market_id: p.market_id,
        order_type: p.order_type,
        order_kind: p.order_kind,
        price: p.order_kind === "Market" ? null : p.price ?? null,
        quantity: p.quantity,
      });
      // Refresh user orders list
      await get().fetch();
      return data;
    } catch (e: any) {
      set({ error: e?.message || "Create order failed" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  cancel: async ({ order_id, market_id }) => {
    set({ loading: true, error: null });
    try {
      const { data } = await api.post<OrderEngineResponse>("/user/orders/cancel", {
        order_id,
        market_id,
      });
      await get().fetch();
      return data;
    } catch (e: any) {
      set({ error: e?.message || "Cancel order failed" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },
}));