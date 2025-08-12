"use client";
import { create } from "zustand";
import { persist } from "zustand/middleware";
import { api } from "@/lib/http";

type AuthState = {
    token: string | null;
    id: string | null;
    email: string | null;
    isAdmin: boolean;
    loading: boolean;
    error: string | null;
    signup: (email: string, password: string) => Promise<void>;
    login: (email: string, password: string, admin?: boolean) => Promise<void>;
    logout: () => void;
  };
  
  export const useAuth = create<AuthState>()(
    persist(
      (set, get) => ({
        token: null,
        id: null,
        email: null,
        isAdmin: false,
        loading: false,
        error: null,

        signup: async (email, password) => {
          set({ loading: true, error: null });
          try {
            await api.post("/signup", { email, password });
            await get().login(email, password);
          } catch (e: any) {
            set({ error: e?.message || "Signup failed" });
            throw e;
          } finally {
            set({ loading: false });
          }
        },
        login: async (email, password, admin = false) => {
          set({ loading: true, error: null });
          try {
            const { data } = await api.post<{ token: string; email: string, id: string }>(admin ? "/admin/login" : "/login", { email, password });
            localStorage.setItem("authToken", data.token);
            set({ token: data.token, email: data.email, isAdmin: !!admin, id: data.id.toString() });
            console.log(data);
          } catch (e: any) {
            set({ error: e?.message || "Login failed" });
            throw e;
          } finally {
            set({ loading: false });
          }
        },
        logout: () => {
          localStorage.removeItem("authToken");
          set({ token: null, email: null, isAdmin: false, error: null });
        },
      }),
      { name: "auth" }
    )
  );