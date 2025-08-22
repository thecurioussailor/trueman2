"use client";
import Markets from "@/components/exchange/Markets";
import HeroCarousel from "@/components/exchange/HeroCarousel";
import Link from "next/link";
import { FaDiscord, FaGithub, FaLinkedin, FaXTwitter } from "react-icons/fa6";
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
          <div className="flex items-center gap-4">
             <span className="text-zinc-400">Made with ❤️ by</span>
            <Link href="https://x.com/sagar11ashutosh" target="_blank" className="hover:text-white">
              <FaXTwitter />  
            </Link>
            <Link href="https://github.com/thecurioussailor" target="_blank" className="hover:text-white"><FaGithub /></Link>
            <Link href="https://discord.gg/Xuj3hdYS" target="_blank" className="hover:text-white"><FaDiscord /></Link>
            <Link href="https://www.linkedin.com/in/ashutosh-sagar-4b2612185/" target="_blank" className="hover:text-white"><FaLinkedin /></Link>
          </div>
        </div>
      </footer>
    </main>
  );
}