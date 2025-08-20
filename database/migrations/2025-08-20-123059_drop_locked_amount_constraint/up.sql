-- Your SQL goes here
-- Drop the constraint that assumes amount is total balance
ALTER TABLE balances DROP CONSTRAINT chk_locked_amount_valid;