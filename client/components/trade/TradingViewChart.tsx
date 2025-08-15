"use client";

import { useEffect, useRef, useId } from "react";

declare global {
  interface Window { TradingView: any }
}

export default function TradingViewChart({
  symbol = "BINANCE:BTCUSDT",
  height = 420,
}: { symbol?: string; height?: number }) {
  const id = useId().replace(/:/g, "_");
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;

    const init = () => {
      if (!mountedRef.current || !window.TradingView) return;
      new window.TradingView.widget({
        container_id: id,
        symbol,
        autosize: true,
        interval: "60",
        timezone: "Etc/UTC",
        theme: "dark",
        style: "1",
        locale: "en",
        hide_side_toolbar: false,
        allow_symbol_change: true,
        withdateranges: true,
      });
    };

    if (window.TradingView) init();
    else {
      const s = document.createElement("script");
      s.src = "https://s3.tradingview.com/tv.js";
      s.async = true;
      s.onload = init;
      document.head.appendChild(s);
    }

    return () => {
      mountedRef.current = false;
      const el = document.getElementById(id);
      if (el) el.innerHTML = "";
    };
  }, [id, symbol]);

  return <div id={id} className="w-full" style={{ height }} />;
}