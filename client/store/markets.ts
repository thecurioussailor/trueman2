import { create } from "zustand";
import { api } from "@/lib/http";
import type { EngineToken } from "@/store/tokens";

export type EngineMarket = {
  id: string;
  symbol: string;
  base_currency: EngineToken;
  quote_currency: EngineToken;
  min_order_size: number;
  tick_size: number;
  is_active: boolean;
  created_at: string;
};

type State = {
  markets: EngineMarket[];
  loading: boolean;
  error: string | null;
  // public
  fetchPublic: () => Promise<void>;
  // admin
  fetchAll: () => Promise<void>;
  create: (p: {
    symbol: string;
    base_currency_id: string;
    quote_currency_id: string;
    min_order_size: number;
    tick_size: number;
  }) => Promise<void>;
  update: (
    id: string,
    p: Partial<{
      symbol: string;
      base_currency_id: string;
      quote_currency_id: string;
      min_order_size: number;
      tick_size: number;
      is_active: boolean;
    }>
  ) => Promise<void>;
  deactivate: (id: string) => Promise<void>;
};

export const useMarkets = create<State>((set, get) => ({
  markets: [],
  loading: false,
  error: null,

  fetchPublic: async () => {
    set({ loading: true, error: null });
    try {
      const { data } = await api.get<{ data: EngineMarket[] }>("/markets");
      set({ markets: data.data ?? [] });
    } catch (e: any) {
      set({ error: e?.message || "Failed to load markets" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  fetchAll: async () => {
    set({ loading: true, error: null });
    try {
      const { data } = await api.get<{ data: EngineMarket[] }>("/admin/markets");
      set({ markets: data.data ?? [] });
    } catch (e: any) {
      set({ error: e?.message || "Failed to load markets" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  create: async (p) => {
    set({ loading: true, error: null });
    try {
      await api.post("/admin/markets", p);
      await get().fetchAll();
    } catch (e: any) {
      set({ error: e?.message || "Create market failed" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  update: async (id, p) => {
    set({ loading: true, error: null });
    try {
      await api.put(`/admin/markets/${id}`, p);
      await get().fetchAll();
    } catch (e: any) {
      set({ error: e?.message || "Update market failed" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  deactivate: async (id) => {
    set({ loading: true, error: null });
    try {
      await api.delete(`/admin/markets/${id}`);
      await get().fetchAll();
    } catch (e: any) {
      set({ error: e?.message || "Deactivate market failed" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },
}));