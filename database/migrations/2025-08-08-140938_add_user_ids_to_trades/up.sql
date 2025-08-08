-- Your SQL goes here
ALTER TABLE trades 
    ADD COLUMN buyer_user_id UUID,
    ADD COLUMN seller_user_id UUID;

CREATE INDEX idx_trades_buyer_user_id ON trades (buyer_user_id);
CREATE INDEX idx_trades_seller_user_id ON trades (seller_user_id);
CREATE INDEX IF NOT EXISTS idx_trades_created_at ON trades (created_at);