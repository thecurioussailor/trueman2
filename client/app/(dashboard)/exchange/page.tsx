"use client";
import Markets from "@/components/exchange/Markets";

export default function ExchangePage() {

  return (
    <main className="min-h-screen bg-[#0b0f14] text-zinc-100">
      {/* BG gradients */}
      <div className="pointer-events-none fixed inset-0 -z-10">
        <div className="absolute -top-40 -left-32 h-[42rem] w-[42rem] rounded-full blur-3xl bg-gradient-to-br from-violet-600/30 to-cyan-400/20" />
        <div className="absolute -top-32 right-0 h-[36rem] w-[36rem] rounded-full blur-3xl bg-gradient-to-tr from-cyan-400/20 to-violet-600/20" />
      </div>
      <Markets/>
    </main>
  );
}