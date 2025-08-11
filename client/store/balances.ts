"use client";

import { create } from "zustand";
import { api } from "@/lib/http";

type EngineBalance = {
  token_id: string;
  available: number;
  locked: number;
}


type Tx = { success: boolean; message: string; new_balance: number | null };

type State = {
  items: EngineBalance[];
  loading: boolean;
  error: string | null;
  lastTx: Tx | null;
  fetch: () => Promise<void>;
  deposit: (token_id: string, amountUnits: number) => Promise<Tx>;
  withdraw: (token_id: string, amountUnits: number) => Promise<Tx>;
};

export const useBalances = create<State>((set, get) => ({
  items: [],
  loading: false,
  error: null,
  lastTx: null,

  fetch: async () => {
    set({ loading: true, error: null });
    try {
      const { data } = await api.get<EngineBalance[]>("/user/balances");
      set({ 
        items: (data  ?? []).map(b => ({
          token_id: b.token_id,
          available: b.available,
          locked: b.locked,
        }))
    });
    } catch (e: any) {
      set({ error: e?.message || "Failed to load balances" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  deposit: async (token_id, amount) => {
    set({ loading: true, error: null });
    try {
      const { data } = await api.post<Tx>("/user/deposit", { token_id, amount });
      set({ lastTx: data });
      await get().fetch();
      return data;
    } catch (e: any) {
      set({ error: e?.message || "Deposit failed" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  withdraw: async (token_id, amount) => {
    set({ loading: true, error: null });
    try {
      const { data } = await api.post<Tx>("/user/withdraw", { token_id, amount });
      set({ lastTx: data });
      await get().fetch();
      return data;
    } catch (e: any) {
      set({ error: e?.message || "Withdraw failed" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },
}));