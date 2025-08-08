-- This file should undo anything in `up.sql`
  ALTER TABLE trades
    ALTER COLUMN buyer_user_id DROP NOT NULL,
    ALTER COLUMN seller_user_id DROP NOT NULL;