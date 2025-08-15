import { create } from "zustand";
import { api } from "@/lib/http";

export type StartSimulatorPayload = {
  market_id: string;
  base_token_id: string;
  quote_token_id: string;
  users: number;
  base_deposit: number;
  quote_deposit: number;
  order_rate_ms: number;
  min_qty: number;
  max_qty: number;
  start_mid: number;
  tick: number;
};

type StartResponse = { queued: boolean; id: string };

type State = {
  loading: boolean;
  error: string | null;
  lastEnqueueId: string | null;
  start: (p: StartSimulatorPayload) => Promise<StartResponse>;
  reset: () => void;
};

export const useSimulator = create<State>((set) => ({
  loading: false,
  error: null,
  lastEnqueueId: null,

  start: async (p) => {
    set({ loading: true, error: null });
    try {
      const { data } = await api.post<StartResponse>("/user/simulator/start", p);
      set({ lastEnqueueId: data?.id ?? null });
      return data;
    } catch (e: any) {
      set({ error: e?.message || "Failed to start simulator" });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  reset: () => set({ error: null, lastEnqueueId: null }),
}));