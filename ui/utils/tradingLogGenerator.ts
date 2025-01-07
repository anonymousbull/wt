import { Decimal } from 'decimal.js';

type TradeState = 'Buy' | 'PendingBuy' | 'BuySuccess' | 'PendingSell' | 'SellSuccess' | 'BuyFailed' | 'SellFailed';

function generateRandomTradingLog(): string {
    const id = Math.floor(Math.random() * 1000000);
    const states: TradeState[] = ['Buy', 'PendingBuy', 'BuySuccess', 'PendingSell', 'SellSuccess', 'BuyFailed', 'SellFailed'];
    const state = states[Math.floor(Math.random() * states.length)];
    const signature = Math.random().toString(36).substring(2, 15);
    const amm = ['raydium', 'orca', 'serum'][Math.floor(Math.random() * 3)];
    const pct = (Math.random() * 20 - 10).toFixed(2); // Range from -10% to +10%

    const signatureUrl = `https://solscan.io/tx/${signature}`;
    const ammUrl = `https://solscan.io/account/${amm}`;

    return `id=${id} state=${state} signature=${signatureUrl} amm=${ammUrl} pct=${pct}`;
}

export function generateTradingLogs(count: number): string[] {
    return Array.from({ length: count }, generateRandomTradingLog);
}

