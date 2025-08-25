// client/components/exchange/HeroCarousel.tsx
"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { ChevronLeft, ChevronRight } from "lucide-react";
import Image from "next/image";

type Slide = {
  title: string;
  subtitle?: string;
  cta?: { label: string; href: string };
  bgClass: string; // Tailwind classes for background
  image: string;
};

const SLIDES: Slide[] = [
  {
    title: "The World is Changing",
    subtitle: "You can't stop things like Bitcoin. It will be everywhere, and the world will have to readjust. World governments will have to readjust.",
    cta: { label: "John McAfee", href: "/exchange?tab=deposit" },
    bgClass:
      "bg-gradient-to-br from-emerald-600/25 to-emerald-400/10",
    image: "/goku3.mp4",
  },
  {
    title: "Welcome to Electricity",
    subtitle: "If you want to find the secrets of the universe, think in terms of energy, frequency and vibration.",
    cta: { label: "Nicola Tesla", href: "/points" },
    bgClass:
      "bg-gradient-to-br from-[#0d1220] via-[#132235] to-[#0b0f14] bg-[url('/hero/season2.svg')] bg-cover bg-center",
    image: "/tesla21.jpg",
  },
  {
    title: "The Future is Here",
    subtitle: "I am late to the party but I am a supporter of Bitcoin.",
    cta: { label: "Elon Musk", href: "/trade/btc-usd" },
    bgClass:
      "bg-gradient-to-tr from-violet-700/30 to-cyan-500/20",
    image: "/elonmusk.jpg",
  },
  {
    title: "Freedom of Money",
    subtitle: "The best time to invest in Bitcoin was yesterday; the second-best time is today.",
    cta: { label: "Anonymous", href: "/exchange?tab=deposit" },
    bgClass:
      "bg-gradient-to-br from-emerald-600/25 to-emerald-400/10",
    image: "/peaky.jpg",
  },
  {
    title: "Rewriting the Rules of Money",
    subtitle: "Bitcoin will do to banks what email did to the postal industry.",
    cta: { label: "Rick Falkvinge", href: "/exchange?tab=deposit" },
    bgClass:
      "bg-gradient-to-br from-emerald-600/25 to-emerald-400/10",
    image: "/happiness.jpg",
  },
  {
    title: "The End of Evolution",
    subtitle: "Gold was the past. Fiat is the present. Bitcoin is the future.",
    cta: { label: "Anonymous", href: "/exchange?tab=deposit" },
    bgClass:
      "bg-gradient-to-br from-emerald-600/25 to-emerald-400/10",
    image: "/evolution.jpg",
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
              className={`min-w-full relative h-72 w-full md:h-96`}
            >
              {s.image.endsWith('.mp4') || s.image.endsWith('.webm') || s.image.endsWith('.mov') ? (
  <div className="absolute inset-0">
    <video
      autoPlay
      loop
      muted
      playsInline
      className="absolute inset-0 w-full h-full object-contain object-right"
    >
      <source src={s.image} type="video/mp4" />
    </video>
    {/* Gradient overlay for video */}
    <div 
      className="absolute inset-0"
      style={{
        background: 'linear-gradient(90deg, #000000 0%, rgba(0,0,0,1) 50%, rgba(0,0,0,0.6) 70%, transparent 100%)'
      }}
    />
  </div>
) : (
  /* Image with gradient overlay */
  <div className="absolute inset-0">
    <div 
      className="absolute inset-0"
      style={{
        backgroundImage: `url(${s.image})`,
        backgroundSize: 'contain',
        backgroundPosition: 'right',
        backgroundRepeat: 'no-repeat',
      }}
    />
    {/* Gradient overlay for image */}
    <div 
      className="absolute inset-0"
      style={{
        background: 'linear-gradient(90deg, #000000 0%, rgba(0,0,0,1) 50%, rgba(0,0,0,0.6) 70%, transparent 100%)'
      }}
    />
  </div>
)}
              
              <div className="relative z-10 flex h-full flex-col justify-center gap-4 px-20 text-left">
                <h2 className="text-3xl font-extrabold md:text-5xl w-1/2">{s.title}</h2>
                {s.subtitle && (
                  <p className="max-w-xl text-zinc-300 text-2xl">"{s.subtitle}"</p>
                )}
                {s.cta && (
                  <div
                    className="inline-flex justify-end w-1/2 pr-8 items-center rounded-lg font-medium text-zinc-100"
                  >
                    - {s.cta.label}
                  </div>
                )}
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