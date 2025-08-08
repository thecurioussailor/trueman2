-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS idx_trades_buyer_user_id;
DROP INDEX IF EXISTS idx_trades_seller_user_id;
-- keep created_at index if shared
ALTER TABLE trades
  DROP COLUMN buyer_user_id,
  DROP COLUMN seller_user_id;