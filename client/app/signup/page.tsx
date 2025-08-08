"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";

const API_BASE = process.env.NEXT_PUBLIC_API_BASE_URL || "http://127.0.0.1:8080";

export default function SignupPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [pw, setPw] = useState("");
  const [cpw, setCpw] = useState("");
  const [loading, setLoading] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [ok, setOk] = useState<string | null>(null);

  const passwordsMatch = pw === cpw;
  const canSubmit = email && pw.length >= 6 && passwordsMatch && !loading;

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!canSubmit) return;
    setErr(null);
    setOk(null);
    setLoading(true);
    try {
      const res = await fetch(`${API_BASE}/signup`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password: pw }),
      });
      if (!res.ok) {
        const msg = await res.text();
        throw new Error(msg || "Signup failed");
      }
      setOk("Account created. Redirecting to login…");
      setTimeout(() => router.push("/login"), 1000);
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
            Create your{" "}
            <span className="bg-gradient-to-r from-violet-400 to-cyan-300 bg-clip-text text-transparent">
              Trueman Exchange
            </span>{" "}
            account
          </h1>
          <p className="mt-2 text-sm text-zinc-400">Start trading in minutes.</p>
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
              className="w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 outline-none ring-0 placeholder:text-zinc-500 focus:border-white/20"
              placeholder="you@example.com"
            />
          </label>

          <label className="mb-4 block">
            <span className="mb-1 block text-sm text-zinc-300">Password</span>
            <input
              type="password"
              autoComplete="new-password"
              required
              minLength={6}
              value={pw}
              onChange={(e) => setPw(e.target.value)}
              className="w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 outline-none focus:border-white/20"
              placeholder="At least 6 characters"
            />
          </label>

          <label className="mb-2 block">
            <span className="mb-1 block text-sm text-zinc-300">Confirm Password</span>
            <input
              type="password"
              autoComplete="new-password"
              required
              minLength={6}
              value={cpw}
              onChange={(e) => setCpw(e.target.value)}
              className="w-full rounded-lg border border-white/10 bg-black/30 px-3 py-2 outline-none focus:border-white/20"
              placeholder="Re-enter password"
            />
          </label>

          {!passwordsMatch && cpw.length > 0 && (
            <div className="mb-2 text-xs font-medium text-rose-400">Passwords do not match</div>
          )}
          {err && <div className="mb-2 rounded-md border border-rose-500/30 bg-rose-500/10 p-2 text-sm text-rose-300">{err}</div>}
          {ok && <div className="mb-2 rounded-md border border-emerald-500/30 bg-emerald-500/10 p-2 text-sm text-emerald-300">{ok}</div>}

          <button
            type="submit"
            disabled={!canSubmit}
            className="mt-3 inline-flex h-11 w-full items-center justify-center rounded-xl bg-gradient-to-r from-violet-500 to-cyan-400 px-5 text-sm font-bold text-black hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-60"
          >
            {loading ? "Creating account…" : "Sign up"}
          </button>

          <div className="mt-3 text-center text-sm text-zinc-400">
            Already have an account?{" "}
            <Link href="/login" className="text-zinc-200 underline-offset-2 hover:underline">
              Log in
            </Link>
          </div>
        </form>
      </div>
    </main>
  );
}