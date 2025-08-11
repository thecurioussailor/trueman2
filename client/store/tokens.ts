import { create } from "zustand";
import { api } from "@/lib/http";

export type EngineToken = {
  id: string;
  symbol: string;
  name: string;
  decimals: number;
  is_active: boolean;
  created_at: string;
}

type State = {
    tokens: EngineToken[];
    loading: boolean;
    error: string | null;
    // public
    fetchPublic: () => Promise<void>;
    // admin
    fetchAll: () => Promise<void>;
    create: (p: { symbol: string; name: string; decimals: number }) => Promise<void>;
    update: (id: string, p: Partial<{ symbol: string; name: string; decimals: number; is_active: boolean }>) => Promise<void>;
    deactivate: (id: string) => Promise<void>;
};

export const useTokens = create<State>((set, get) => ({
    tokens: [],
    loading: false,
    error: null,
  
    fetchPublic: async () => {
      set({ loading: true, error: null });
      try {
        const { data } = await api.get<{ data: EngineToken[] }>("/tokens");
        set({ tokens: data.data ?? [] });
      } catch (e: any) {
        set({ error: e?.message || "Failed to load tokens" });
        throw e;
      } finally {
        set({ loading: false });
      }
    },
  
    // Admin endpoints (mounted under /admin)
    fetchAll: async () => {
      set({ loading: true, error: null });
      try {
        const { data } = await api.get<{ data: EngineToken[] }>("/admin/tokens");
        set({ tokens: data.data ?? [] });
      } catch (e: any) {
        set({ error: e?.message || "Failed to load tokens" });
        throw e;
      } finally {
        set({ loading: false });
      }
    },
  
    create: async (p) => {
      set({ loading: true, error: null });
      try {
        await api.post("/admin/tokens", p);
        await get().fetchAll();
      } catch (e: any) {
        set({ error: e?.message || "Create token failed" });
        throw e;
      } finally {
        set({ loading: false });
      }
    },
  
    update: async (id, p) => {
      set({ loading: true, error: null });
      try {
        await api.put(`/admin/tokens/${id}`, p);
        await get().fetchAll();
      } catch (e: any) {
        set({ error: e?.message || "Update token failed" });
        throw e;
      } finally {
        set({ loading: false });
      }
    },
  
    deactivate: async (id) => {
      set({ loading: true, error: null });
      try {
        await api.delete(`/admin/tokens/${id}`);
        await get().fetchAll();
      } catch (e: any) {
        set({ error: e?.message || "Deactivate token failed" });
        throw e;
      } finally {
        set({ loading: false });
      }
    },
}));