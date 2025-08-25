"use client";
import Markets from "@/components/exchange/Markets";
import Link from "next/link";
import { FaDiscord, FaGithub, FaLinkedin, FaXTwitter } from "react-icons/fa6";
import { Gi3dGlasses } from "react-icons/gi";

export default function Home() {
  return (
    <main className="relative min-h-screen text-zinc-100 bg-[#0b0f14] overflow-x-hidden">
      {/* BG gradients */}
      <div className="pointer-events-none absolute inset-0 -z-10">
        <div className="absolute -top-40 -left-32 h-[42rem] w-[42rem] rounded-full blur-3xl bg-gradient-to-br from-violet-600/30 to-cyan-400/20" />
        <div className="absolute -top-32 right-0 h-[36rem] w-[36rem] rounded-full blur-3xl bg-gradient-to-tr from-cyan-400/20 to-violet-600/20" />
      </div>

      {/* Header */}
      <header className="sticky top-0 z-20 border-b border-white/10 bg-black/20 backdrop-blur">
        <div className="mx-auto flex h-16 max-w-6xl items-center justify-between px-4">
          <div className="flex items-center gap-3 font-semibold">
            <Gi3dGlasses size={24}/>
            <span className="text-xl font-bold">Trueman</span>
          </div>
          <nav className="hidden md:flex items-center gap-6 text-sm font-semibold text-zinc-300">
            <Link href="/user/markets" className="hover:text-white">Features</Link>
            <Link href="/user/orders" className="hover:text-white">Markets</Link>
            <Link href="/user/orders" className="hover:text-white">Leaderboard</Link>
          </nav>
          <div className="flex items-center gap-2">
            <Link href="/login" className="flex justify-center items-center h-9 rounded-lg border border-white/15 px-3 text-sm text-white hover:bg-white/5">
              Log in
            </Link>
            <Link
              href="/signup"
              className="flex justify-center items-center h-9 rounded-lg bg-gradient-to-r from-violet-500 to-cyan-400 px-3 text-sm font-semibold text-black hover:brightness-110"
            >
              Sign up
            </Link>
          </div>
        </div>
      </header>

      {/* Hero */}
      <section className="px-4 py-20">
        <div className="mx-auto max-w-5xl py-8 text-center">
          <div className="inline-block rounded-full bg-violet-600/20 px-3 py-1 text-xs font-semibold text-violet-300 mb-4">
            🚀 Demo Project – Risk-free trading simulator
          </div>
          <div className="mt-2 overflow-hidden whitespace-nowrap text-xs text-zinc-300">
            <span className="mx-4 tracking-wider">BTC/USDC: $65,200 ▲1.2%</span>
            <span className="mx-4 tracking-wider">ETH/USDC: $3,500 ▼0.8%</span>
            <span className="mx-4 tracking-wider">SOL/USDC: $180 ▲3.5%</span>
          </div>
          <h1 className="mx-auto max-w-4xl text-4xl font-extrabold leading-tight sm:text-6xl">
            Trade crypto with confidence on{" "}
            <span className="bg-gradient-to-r from-violet-400 to-cyan-300 bg-clip-text text-transparent">
              Trueman Exchange
            </span>
          </h1>
          <p className="mx-auto mt-4 max-w-2xl text-zinc-300">
            A fast, secure, and intuitive centralized exchange. Low fees, deep liquidity, and a modern UI inspired by the best.
          </p>
          <div className="mt-6 flex flex-wrap justify-center gap-3">
            <Link
              href="/signup"
              className="flex justify-center items-center h-11 rounded-xl bg-gradient-to-r from-violet-500 to-cyan-400 px-5 text-sm font-bold text-black hover:brightness-110"
            >
              Get Started
            </Link>
            <Link
              href="/login"
              className="flex justify-center items-center h-11 rounded-xl border border-white/15 bg-white/5 px-5 text-sm font-semibold text-white hover:bg-white/10"
            >
              Try Demo Trading
            </Link>
          </div>

          {/* Stats */}
          <div className="mt-10 grid grid-cols-1 gap-3 sm:grid-cols-3">
            <Stat label="Maker Fee" value="0.05%" />
            <Stat label="Uptime" value="99.99%" />
            <Stat label="Match Latency" value="100ms" />
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="px-4 py-10">
        <div className="mx-auto max-w-6xl">
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-4">
            <Card
              title="Deep Liquidity"
              desc="Tight spreads across major pairs with robust orderbooks."
              icon={<IconBooks />}
            />
            <Card
              title="Advanced Security"
              desc="Hardened infra, best-practice auth, and continuous monitoring."
              icon={<IconShield />}
            />
            <Card
              title="Pro Tools"
              desc="Market/limit orders, live charts, order history, and more."
              icon={<IconTools />}
            />
            <Card
              title="Low Fees"
              desc="Transparent maker/taker fees with volume discounts."
              icon={<IconStar />}
            />
          </div>
        </div>
      </section>

      {/* Top Markets */}
      <section className="px-4 py-10">
        <div className="mx-auto max-w-6xl">
        <h2 className="text-2xl font-bold pl-4">Top Markets</h2>
          <Markets/>
        </div>
      </section>
      <section className="px-4 py-10">
        <div className="mx-auto max-w-6xl">
          <h2 className="text-2xl font-bold mb-4">Leaderboard (Coming Soon)</h2>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <div className="p-4 rounded-xl border border-white/10 bg-white/5">
              <div className="font-bold">Alice</div>
              <div className="text-sm text-green-400">+45% PnL</div>
            </div>
            <div className="p-4 rounded-xl border border-white/10 bg-white/5">
              <div className="font-bold">Bob</div>
              <div className="text-sm text-green-400">+32% PnL</div>
            </div>
            <div className="p-4 rounded-xl border border-white/10 bg-white/5">
              <div className="font-bold">Charlie</div>
              <div className="text-sm text-green-400">+20% PnL</div>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-white/10 bg-black/20">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-4 py-6 text-sm text-zinc-300">
          <div>© {new Date().getFullYear()} Trueman Exchange</div>
          <div className="flex items-center gap-4">
            <Link href="https://x.com/sagar11ashutosh" target="_blank" className="hover:text-white"><FaXTwitter /></Link>
            <Link href="https://github.com/thecurioussailor" target="_blank" className="hover:text-white"><FaGithub /></Link>
            <Link href="https://www.linkedin.com/in/ashutosh-sagar-4b2612185/" target="_blank" className="hover:text-white"><FaLinkedin /></Link>
            <Link
              href="https://discord.gg/CZQ4rjNF"
              target="_blank"
              className="flex items-center gap-2 rounded-lg bg-gradient-to-r from-violet-500 to-cyan-400 px-3 py-1 text-sm font-semibold text-black hover:brightness-110"
            >
              <FaDiscord /> Join Discord
            </Link>
          </div>
        </div>
      </footer>


    </main>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-5 text-center">
      <div className="text-xl font-extrabold">{value}</div>
      <div className="mt-1 text-xs text-zinc-400">{label}</div>
    </div>
  );
}

function Card({
  title,
  desc,
  icon,
}: {
  title: string;
  desc: string;
  icon: React.ReactNode;
}) {
  return (
    <div className="h-full rounded-2xl border border-white/10 bg-black/5 bg-gradient-to-b from-white/5 to-white/1 p-5">
      <div className="mb-3 text-zinc-300">{icon}</div>
      <div className="mb-1 font-bold">{title}</div>
      <div className="text-sm text-zinc-300">{desc}</div>
    </div>
  );
}

function IconBooks() {
  return <svg viewBox="0 0 24 24" width="24" height="24"><path fill="currentColor" d="M3 3h18v2H3V3zm2 4h14v2H5V7zm-2 4h18v2H3v-2zm2 4h10v2H5v-2zm-2 4h18v2H3v-2z"/></svg>;
}
function IconShield() {
  return <svg viewBox="0 0 24 24" width="24" height="24"><path fill="currentColor" d="M12 2l8 4v6c0 5-3.4 9.7-8 10-4.6-.3-8-5-8-10V6l8-4zM7 10h10v2H7v-2z"/></svg>;
}
function IconTools() {
  return <svg viewBox="0 0 24 24" width="24" height="24"><path fill="currentColor" d="M4 4h16v2H4V4zm2 4h6v2H6V8zm0 4h12v2H6v-2zm0 4h8v2H6v-2z"/></svg>;
}
function IconStar() {
  return <svg viewBox="0 0 24 24" width="24" height="24"><path fill="currentColor" d="M12 1l3 5 6 .9-4.3 4.2 1 6-5.7-3-5.7 3 1-6L3 6.9 9 6z"/></svg>;
}