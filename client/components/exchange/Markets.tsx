import Link from "next/link";
import { useEffect, useState } from "react";
import { useMarkets } from "@/store/markets";
import Image from "next/image";

interface BinanceTicker {
  symbol: string;
  priceChange: number;
  priceChangePercent: number;
  lastPrice: number;
  volume: number;
  quoteVolume: number;
  image: string;
}
const Markets = () => {
  const { markets, loading, error, fetchPublic } = useMarkets();
  const [binanceData, setBinanceData] = useState<Record<string, BinanceTicker>>({});
  const [loadingBinance, setLoadingBinance] = useState(true);
  useEffect(() => {
    fetchPublic();
  }, [fetchPublic]);

  useEffect(() => {
    if (markets.length > 0) {
      fetchBinanceData();
    }
  }, [markets]);

  const fetchBinanceData = async () => {
    setLoadingBinance(true);
    try {
      // Convert market symbols to Binance format (e.g., BTC-USDT -> BTCUSDT)
      const binanceSymbols = markets.map(market => {
        const [base, quote] = market.symbol.split('-');
        return `${base.toUpperCase()}${quote.toUpperCase()}`;
      });

      const symbolsParam = JSON.stringify(binanceSymbols);
      const response = await fetch(
        `https://api.binance.com/api/v3/ticker/24hr?symbols=${symbolsParam}`,
        { cache: 'no-store' }
      );

      if (!response.ok) throw new Error('Failed to fetch Binance data');
      
      const data: BinanceTicker[] = await response.json();
      
      // Create a map for easy lookup
      const dataMap = data.reduce((acc, ticker) => {
        acc[ticker.symbol] = {
          ...ticker,
          image: `https://backpack.exchange/_next/image?url=%2Fcoins%2F${ticker.symbol.replace("USDC", "").toLowerCase()}.png&w=96&q=95`
        };
        return acc;
      }, {} as Record<string, BinanceTicker>);
      
      setBinanceData(dataMap);
    } catch (error) {
      console.error('Error fetching Binance data:', error);
    } finally {
      setLoadingBinance(false);
    }
  };

  const formatPrice = (price: string) => {
    const num = parseFloat(price);
    return `$${num.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  };

  const formatVolume = (volume: string) => {
    const num = parseFloat(volume);
    if (num >= 1e9) return `$${(num / 1e9).toFixed(1)}B`;
    if (num >= 1e6) return `$${(num / 1e6).toFixed(1)}M`;
    if (num >= 1e3) return `$${(num / 1e3).toFixed(1)}K`;
    return `$${num.toFixed(0)}`;
  };

  const formatChange = (change: string) => {
    const changeNum = parseFloat(change);
    const isPositive = changeNum >= 0;
    return (
      <span className={isPositive ? "text-green-400" : "text-red-400"}>
        {isPositive ? "+" : ""}{changeNum.toFixed(2)}%
      </span>
    );
  };

  // Simple market cap estimation (you'd want real supply data in production)
  const estimateMarketCap = (symbol: string, price: string) => {
    const priceNum = parseFloat(price);
    const supplies: Record<string, number> = {
      'BTCUSDT': 19.7e6,
      'ETHUSDT': 120e6,
      'SOLUSDT': 400e6,
    };
    
    const supply = supplies[symbol] || 1e6;
    const marketCap = priceNum * supply;
    
    if (marketCap >= 1e12) return `$${(marketCap / 1e12).toFixed(1)}T`;
    if (marketCap >= 1e9) return `$${(marketCap / 1e9).toFixed(1)}B`;
    if (marketCap >= 1e6) return `$${(marketCap / 1e6).toFixed(1)}M`;
    return `$${marketCap.toFixed(0)}`;
  };

  return (
    <div className="mx-auto max-w-7xl px-4 py-6">

        {/* Markets table */}
        <div className="overflow-hidden rounded-xl border border-white/10 bg-black/5">
          <table className="w-full border-collapse text-sm">
            <thead className="bg-black/30 text-zinc-300">
              <tr>
                <th className="px-6 py-6 text-left font-semibold text-zinc-400">Name</th>
                <th className="px-6 py-6 text-right font-semibold text-zinc-400">Price</th>
                <th className="px-6 py-6 text-right font-semibold text-zinc-400">24h Volume</th>
                <th className="px-6 py-6 text-right font-semibold text-zinc-400">Market Cap</th>
                <th className="px-6 py-6 text-right font-semibold text-zinc-400">24h Change</th>
                <th className="px-6 py-6 text-right font-semibold text-zinc-400"></th>
              </tr>
            </thead>
            <tbody>
            {loading && (
              <tr>
                <td className="px-4 py-8 text-center text-zinc-400" colSpan={5}>
                  Loading markets...
                </td>
              </tr>
            )}
            {!loading &&
              markets.map((m) => {
                // Public markets API does not include price/change/volume; placeholders for now
                const symbolDisplay = m.symbol.replace("-", "/");
                const name = m.base_currency.name || m.symbol.split("-")[0];
                
                // Get Binance symbol format
                const [base, quote] = m.symbol.split('-');
                const binanceSymbol = `${base.toUpperCase()}${quote.toUpperCase()}`;
                const tickerData = binanceData[binanceSymbol];
                return (
                  <tr key={m.id} className="border-t border-white/10 hover:bg-white/5 transition-colors">
                    <td className="px-6 py-3 font-semibold">
                      <div className="flex gap-4">
                        <div className="flex items-center gap-2">
                          {tickerData && <Image src={tickerData?.image} alt={name} width={32} height={32} />}
                        </div>
                        <div className="flex flex-col gap-1">
                          <div className="text-white">{name}</div>
                          <div className="text-zinc-400 text-xs">{symbolDisplay}</div>
                        </div>
                      </div>
                    </td>
                    <td className="px-6 py-3 font-semibold text-white text-right">
                      {loadingBinance ? (
                        <div className="animate-pulse bg-zinc-700 h-4 w-20 rounded"></div>
                      ) : tickerData ? (
                        formatPrice(tickerData.lastPrice.toString())
                      ) : (
                        "-"
                      )}
                    </td>
                    <td className="px-6 py-3 text-white text-right">
                      {loadingBinance ? (
                        <div className="animate-pulse bg-zinc-700 h-4 w-16 rounded"></div>
                      ) : tickerData ? (
                        formatVolume(tickerData.quoteVolume.toString())
                      ) : (
                        "-"
                      )}
                    </td>
                    <td className="px-6 py-3 text-white text-right">
                      {loadingBinance ? (
                        <div className="animate-pulse bg-zinc-700 h-4 w-16 rounded"></div>
                      ) : tickerData ? (
                        estimateMarketCap(binanceSymbol, tickerData.lastPrice.toString())
                      ) : (
                        "-"
                      )}
                    </td>
                    <td className="px-6 py-3 text-right">
                      {loadingBinance ? (
                        <div className="animate-pulse bg-zinc-700 h-4 w-12 rounded"></div>
                      ) : tickerData ? (
                        formatChange(tickerData.priceChangePercent.toString())
                      ) : (
                        "-"
                      )}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <Link
                        href={{
                          pathname: `/trade/${m.symbol}`,
                          query: { id: m.id },
                        }}
                        className="inline-flex h-8 items-center rounded-lg border border-white/10 bg-white/5 px-3 text-xs font-semibold hover:bg-white/10 transition-colors"
                      >
                        Trade
                      </Link>
                    </td>
                  </tr>
                );
              })}
            {!loading && markets.length === 0 && (
              <tr>
                <td className="px-4 py-8 text-center text-zinc-400" colSpan={5}>
                  No markets found
                </td>
              </tr>
            )}
            </tbody>
          </table>
        </div>
    </div>
  )
}

export default Markets