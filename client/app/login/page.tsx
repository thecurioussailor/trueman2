"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";

const API_BASE = process.env.NEXT_PUBLIC_API_BASE_URL || "http://127.0.0.1:8080";

export default function LoginPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [pw, setPw] = useState("");
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const canSubmit = email && pw && !loading;

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!canSubmit) return;
    setErr(null);
    setLoading(true);
    try {
      const res = await fetch(`${API_BASE}/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password: pw }),
      });
      if (!res.ok) {
        const msg = await res.text();
        throw new Error(msg || "Login failed");
      }
      // token may be raw string or { token: string }
      let token: string | undefined;
      const text = await res.text();
      try {
        const data = JSON.parse(text);
        token = typeof data === "string" ? data : data?.token;
      } catch {
        token = text;
      }
      if (!token) throw new Error("Invalid login response");
      // Persist and go to app
      localStorage.setItem("authToken", token);
      router.push("/user/markets");
    } catch (e: any) {
      setErr(e?.message || "Something went wrong");
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="relative min-h-screen bg-[#0b0f14] text-zinc-100">
      {/* BG */}
      <div className="pointer-events-none absolute inset-0 -z-10">
        <div className="absolute -top-40 -left-32 h-[42rem] w-[42rem] rounded-full blur-3xl bg-gradient-to-br from-violet-600/30 to-cyan-400/20" />
        <div className="absolute -top-32 right-0 h-[36rem] w-[36rem] rounded-full blur-3xl bg-gradient-to-tr from-cyan-400/20 to-violet-600/20" />
      </div>

      <div className="mx-auto flex min-h-screen max-w-md flex-col justify-center px-4">
        <div className="mb-8 text-center">
          <h1 className="text-3xl font-extrabold">
            Welcome back to{" "}
            <span className="bg-gradient-to-r from-violet-400 to-cyan-300 bg-clip-text text-transparent">
              Trueman Exchange
            </span>
          </h1>
          <p className="mt-2 text-sm text-zinc-400">Log in to continue trading.</p>
        </div>

        <form
          onSubmit={onSubmit}
          className="rounded-2xl border border-white/10 bg-white/5 p-6 shadow-2xl backdrop-blur"
        >
          <label className="mb-4 block">
            <span className="mb-1 block text-sm text-zinc-300">Email</span>
            <input
              type="email"
              autoComplete="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 outline-none placeholder:text-zinc-500 focus:border-white/20"
              placeholder="you@example.com"
            />
          </label>

          <label className="mb-2 block">
            <span className="mb-1 block text-sm text-zinc-300">Password</span>
            <input
              type="password"
              autoComplete="current-password"
              required
              value={pw}
              onChange={(e) => setPw(e.target.value)}
              className="w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 outline-none focus:border-white/20"
              placeholder="Your password"
            />
          </label>

          {err && (
            <div className="mb-2 rounded-md border border-rose-500/30 bg-rose-500/10 p-2 text-sm text-rose-300">
              {err}
            </div>
          )}

          <button
            type="submit"
            disabled={!canSubmit}
            className="mt-3 inline-flex h-11 w-full items-center justify-center rounded-xl bg-gradient-to-r from-violet-500 to-cyan-400 px-5 text-sm font-bold text-black hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-60"
          >
            {loading ? "Signing in…" : "Log in"}
          </button>

          <div className="mt-3 text-center text-sm text-zinc-400">
            New here?{" "}
            <Link href="/signup" className="text-zinc-200 underline-offset-2 hover:underline">
              Create an account
            </Link>
          </div>
        </form>
      </div>
    </main>
  );
}