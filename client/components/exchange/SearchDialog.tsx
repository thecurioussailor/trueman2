"use client"
import { useMarkets } from "@/store/markets";
import { useEffect, useState, useMemo } from "react";
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

interface SearchDialogProps {
    searchRef: React.RefObject<HTMLDivElement>;
    handleMarketClick: (symbol: string, id: string) => void;
    query: string;
}

const SearchDialog = ({ searchRef, handleMarketClick, query }: SearchDialogProps) => {
    const { markets, fetchPublic } = useMarkets();
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

    // Filter markets based on query
    const filteredMarkets = useMemo(() => {
        if (!query.trim()) return markets.slice(0, 8); // Show first 8 when no query
        
        const searchTerm = query.trim().toLowerCase();
        return markets.filter(market => {
            const symbol = market.symbol.toLowerCase();
            const baseCurrency = market.base_currency?.name?.toLowerCase() || '';
            const symbolDisplay = market.symbol.replace('-', '/').toLowerCase();
            
            return symbol.includes(searchTerm) || 
                   baseCurrency.includes(searchTerm) || 
                   symbolDisplay.includes(searchTerm);
        });
    }, [markets, query]);

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

    const formatChange = (change: string) => {
        const changeNum = parseFloat(change);
        const isPositive = changeNum >= 0;
        return (
            <span className={isPositive ? "text-green-400" : "text-red-400"}>
                {isPositive ? "+" : ""}{changeNum.toFixed(2)}%
            </span>
        );
    };

    return (
        <div ref={searchRef} className="absolute top-full left-0 right-0 mt-2 bg-zinc-900 border border-white/10 rounded-lg shadow-xl max-h-80 overflow-y-auto z-50">
            {filteredMarkets.length > 0 ? (
                filteredMarkets.map((market) => {
                    const symbolDisplay = market.symbol.replace("-", "/");
                    const name = market.base_currency?.name || market.symbol.split("-")[0];
                    
                    // Get Binance symbol format
                    const [base, quote] = market.symbol.split('-');
                    const binanceSymbol = `${base.toUpperCase()}${quote.toUpperCase()}`;
                    const tickerData = binanceData[binanceSymbol];

                    return (
                        <button
                            key={market.symbol}
                            onClick={() => {
                                console.log(market.symbol);
                                handleMarketClick(market.symbol, market.id);
                            }}
                            className="w-full px-4 py-3 text-left cursor-pointer hover:bg-white/5 transition-colors border-b border-white/5 last:border-b-0"
                        >
                            <div className="flex items-center justify-between">
                                <div className="flex items-center gap-3">
                                    <div className="flex items-center gap-2">
                                        {tickerData && (
                                            <Image 
                                                src={tickerData.image} 
                                                alt={name} 
                                                width={24} 
                                                height={24} 
                                                className="rounded-full"
                                            />
                                        )}
                                    </div>
                                    <div className="flex flex-col items-start">
                                        <span className="font-semibold text-white text-sm">
                                            {name}
                                        </span>
                                        <span className="text-xs text-zinc-400">
                                            {symbolDisplay}
                                        </span>
                                    </div>
                                </div>
                                <div className="flex flex-col items-end gap-1">
                                    <span className="font-medium text-white text-sm">
                                        {loadingBinance ? (
                                            <div className="animate-pulse bg-zinc-700 h-3 w-16 rounded"></div>
                                        ) : tickerData ? (
                                            formatPrice(tickerData.lastPrice.toString())
                                        ) : (
                                            "-"
                                        )}
                                    </span>
                                    <span className="text-xs">
                                        {loadingBinance ? (
                                            <div className="animate-pulse bg-zinc-700 h-3 w-12 rounded"></div>
                                        ) : tickerData ? (
                                            formatChange(tickerData.priceChangePercent.toString())
                                        ) : (
                                            "-"
                                        )}
                                    </span>
                                </div>
                            </div>
                        </button>
                    );
                })
            ) : (
                <div className="px-4 py-3 text-zinc-400 text-sm">
                    {query.trim() ? `No markets found for "${query}"` : "No markets available"}
                </div>
            )}
        </div>
    );
};

export default SearchDialog;