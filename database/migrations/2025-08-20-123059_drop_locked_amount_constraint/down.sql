-- This file should undo anything in `up.sql`
-- Re-add the constraint (only use if you change your mind)
ALTER TABLE balances ADD CONSTRAINT chk_locked_amount_valid 
    CHECK (locked_amount <= amount);