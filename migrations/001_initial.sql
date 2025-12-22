-- Create ultra-simplified single shares table with JSONB
CREATE TABLE IF NOT EXISTS shares (
    id TEXT PRIMARY KEY,
    secret TEXT NOT NULL,
    session_id TEXT NOT NULL,
    
    -- Store the current state directly as JSON array
    data JSONB DEFAULT '[]',
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_shares_session_id ON shares(session_id);
CREATE INDEX IF NOT EXISTS idx_shares_created_at ON shares(created_at);
CREATE INDEX IF NOT EXISTS idx_shares_updated_at ON shares(updated_at);

-- Create GIN index for JSONB field to support efficient JSON queries
CREATE INDEX IF NOT EXISTS idx_shares_data_gin ON shares USING GIN(data);