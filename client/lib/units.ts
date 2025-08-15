export const toAtomic = (v: number, d: number) => Math.round(v * 10 ** d);

export const fromAtomic = (v: number, d: number) => v / 10 ** d;

export const quantizePrice = (priceAtomic: number, tickSize: number, side?: 'buy'|'sell') => {
  const q = priceAtomic / tickSize;
  const n = side === 'sell' ? Math.ceil(q) : side === 'buy' ? Math.floor(q) : Math.round(q);
  return n * tickSize;
};