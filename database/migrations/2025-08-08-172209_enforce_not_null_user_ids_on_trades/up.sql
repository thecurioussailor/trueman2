-- Your SQL goes here
  ALTER TABLE trades
    ALTER COLUMN buyer_user_id SET NOT NULL,
    ALTER COLUMN seller_user_id SET NOT NULL;