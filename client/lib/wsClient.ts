// client/lib/wsClient.ts
type Feed = 'depth' | 'ticker' | 'trades';

type WsEvent =
  | { type: 'event'; channel: string; payload: any }
  | { type: 'info'; message: string }
  | Record<string, any>;

type Listener = (msg: WsEvent) => void;

class WsClient {
  private ws: WebSocket | null = null;
  private listeners = new Set<Listener>();
  private url = 'ws://localhost:9000/ws';
  private wantSubscribe: { marketId?: string; feeds?: Feed[] } = {};
  private reconnectTimer?: number;

  addListener = (fn: Listener) => { this.listeners.add(fn); return () => this.listeners.delete(fn); };

  private notify = (msg: WsEvent) => this.listeners.forEach(l => l(msg));

  connect = () => {
    if (typeof window === 'undefined' || this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) return;

    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      if (this.wantSubscribe.marketId && this.wantSubscribe.feeds?.length) {
        this.subscribe(this.wantSubscribe.marketId!, this.wantSubscribe.feeds!);
      }
    };

    this.ws.onmessage = (e) => {
      try { this.notify(JSON.parse(e.data)); } catch { /* ignore */ }
    };

    this.ws.onclose = () => {
      this.ws = null;
      window.clearTimeout(this.reconnectTimer);
      this.reconnectTimer = window.setTimeout(this.connect, 1000);
    };

    this.ws.onerror = () => { this.ws?.close(); };
  };

  subscribe = (marketId: string, feeds: Feed[] = ['depth', 'ticker', 'trades']) => {
    this.wantSubscribe = { marketId, feeds };
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) { this.connect(); return; }
    this.ws.send(JSON.stringify({ action: 'subscribe', market_id: marketId, feeds }));
  };

  unsubscribe = () => {
    this.wantSubscribe = {};
    // Optional: implement server-side unsubscribe if supported
  };
}

export const wsClient = new WsClient();