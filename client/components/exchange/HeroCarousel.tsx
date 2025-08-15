// client/components/exchange/HeroCarousel.tsx
"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { ChevronLeft, ChevronRight } from "lucide-react";

type Slide = {
  title: string;
  subtitle?: string;
  cta?: { label: string; href: string };
  bgClass: string; // Tailwind classes for background
};

const SLIDES: Slide[] = [
  {
    title: "Welcome to Season 2",
    subtitle: "Earn points to move up the ranks. Every action counts.",
    cta: { label: "View Points", href: "/points" },
    bgClass:
      "bg-gradient-to-br from-[#0d1220] via-[#132235] to-[#0b0f14] bg-[url('/hero/season2.svg')] bg-cover bg-center",
  },
  {
    title: "Zero-Fee Spot on BTC/ETH",
    subtitle: "Trade top markets with deep liquidity.",
    cta: { label: "Start Trading", href: "/trade/btc-usd" },
    bgClass:
      "bg-gradient-to-tr from-violet-700/30 to-cyan-500/20",
  },
  {
    title: "Deposit to get started",
    subtitle: "Onboard in seconds with crypto deposits.",
    cta: { label: "Deposit", href: "/exchange?tab=deposit" },
    bgClass:
      "bg-gradient-to-br from-emerald-600/25 to-emerald-400/10",
  },
];

export default function HeroCarousel() {
  const [index, setIndex] = useState(0);
  const [paused, setPaused] = useState(false);

  useEffect(() => {
    if (paused) return;
    const id = setInterval(() => {
      setIndex((i) => (i + 1) % SLIDES.length);
    }, 5000);
    return () => clearInterval(id);
  }, [paused]);

  const go = (i: number) => setIndex((i + SLIDES.length) % SLIDES.length);

  return (
    <section
      role="region"
      aria-label="Promotions"
      className="mx-auto mb-6 max-w-7xl px-4"
      onMouseEnter={() => setPaused(true)}
      onMouseLeave={() => setPaused(false)}
    >
      <div className="relative overflow-hidden rounded-2xl border border-white/10">
        <ul
          className="flex transition-transform duration-500 will-change-transform"
          style={{ transform: `translateX(-${index * 100}%)` }}
        >
          {SLIDES.map((s, i) => (
            <li
              key={i}
              className={`min-w-full ${s.bgClass}`}
            >
              <div className="relative h-72 w-full md:h-96">
                <div className="absolute inset-0 bg-black/20" />
                <div className="relative z-10 flex h-full flex-col justify-center gap-4 px-20 text-left">
                  <h2 className="text-3xl font-extrabold md:text-5xl">{s.title}</h2>
                  {s.subtitle && (
                    <p className="max-w-xl text-zinc-300">{s.subtitle}</p>
                  )}
                  {s.cta && (
                    <Link
                      href={s.cta.href}
                      className="inline-flex w-fit items-center rounded-lg bg-white/10 px-4 py-2 font-medium text-zinc-100 ring-1 ring-white/15 hover:bg-white/15"
                    >
                      {s.cta.label}
                    </Link>
                  )}
                </div>
              </div>
            </li>
          ))}
        </ul>

        <button
          aria-label="Previous"
          onClick={() => go(index - 1)}
          className="absolute left-3 top-1/2 -translate-y-1/2 rounded-full cursor-pointer p-2 text-zinc-100 hover:bg-black/50"
        >
          <ChevronLeft size={30} />
        </button>
        <button
          aria-label="Next"
          onClick={() => go(index + 1)}
          className="absolute right-3 top-1/2 -translate-y-1/2 rounded-full cursor-pointer p-2 text-zinc-100 hover:bg-black/50"
        >
          <ChevronRight size={30} />
        </button>

        <div className="pointer-events-none absolute bottom-3 left-1/2 flex -translate-x-1/2 gap-2">
          {SLIDES.map((_, i) => (
            <span
              key={i}
              className={`h-2 w-2 rounded-full ${i === index ? "bg-white" : "bg-white/40"}`}
            />
          ))}
        </div>
      </div>
    </section>
  );
}