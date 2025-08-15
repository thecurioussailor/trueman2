"use client";
import Markets from "@/components/exchange/Markets";
import HeroCarousel from "@/components/exchange/HeroCarousel";
import Link from "next/link";
import { FaXTwitter } from "react-icons/fa6";
export default function ExchangePage() {

  return (
    <main className="min-h-screen bg-[#0b0f14] text-zinc-100">
      {/* BG gradients */}
      <div className="pointer-events-none fixed inset-0 -z-10">
        <div className="absolute -top-40 -left-32 h-[42rem] w-[42rem] rounded-full blur-3xl bg-gradient-to-br from-violet-600/30 to-cyan-400/20" />
        <div className="absolute -top-32 right-0 h-[36rem] w-[36rem] rounded-full blur-3xl bg-gradient-to-tr from-cyan-400/20 to-violet-600/20" />
      </div>
      <div className="py-10">
        <HeroCarousel />
      </div>
      <Markets/>
      {/* Footer */}
      <footer className="border-t border-white/10 bg-black/20">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-4 py-6 text-sm text-zinc-300">
          <div>© {new Date().getFullYear()} Trueman Exchange</div>
          <div className="flex gap-4">  
            <Link href="https://x.com/sagar11ashutosh" target="_blank" rel="noreferrer" className="hover:text-white"><FaXTwitter /></Link>
            <Link href="/privacy" className="hover:text-white">Privacy</Link>
            <Link href="/fees" className="hover:text-white">Fees</Link>
            <Link href="/support" className="hover:text-white">Support</Link>
          </div>
        </div>
      </footer>
    </main>
  );
}