-- Your SQL goes here
CREATE TABLE balances (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_id UUID NOT NULL REFERENCES tokens(id) ON DELETE CASCADE,
    amount BIGINT NOT NULL DEFAULT 0 CHECK (amount >= 0),
    locked_amount BIGINT NOT NULL DEFAULT 0 CHECK (locked_amount >= 0),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    
    -- Ensure each user has only one balance record per token
    UNIQUE(user_id, token_id)
);

-- Indexes for efficient queries
CREATE INDEX idx_balances_user_id ON balances(user_id);
CREATE INDEX idx_balances_token_id ON balances(token_id);
CREATE INDEX idx_balances_user_token ON balances(user_id, token_id);

-- Add constraint to ensure locked_amount doesn't exceed available amount
ALTER TABLE balances ADD CONSTRAINT chk_locked_amount_valid 
    CHECK (locked_amount <= amount);