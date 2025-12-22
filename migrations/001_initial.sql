-- Create shares table
CREATE TABLE IF NOT EXISTS shares (
    id TEXT PRIMARY KEY,
    secret TEXT NOT NULL,
    session_id TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create share_events table for event sourcing
CREATE TABLE IF NOT EXISTS share_events (
    id BIGSERIAL PRIMARY KEY,
    share_id TEXT NOT NULL,
    event_key TEXT NOT NULL,
    data TEXT NOT NULL, -- JSON string
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (share_id) REFERENCES shares(id) ON DELETE CASCADE
);

-- Create share_compactions table for compacted data
CREATE TABLE IF NOT EXISTS share_compactions (
    share_id TEXT PRIMARY KEY,
    event_key TEXT, -- The last event key included in compaction
    data TEXT NOT NULL, -- JSON array of ShareData
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (share_id) REFERENCES shares(id) ON DELETE CASCADE
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_share_events_share_id ON share_events(share_id);
CREATE INDEX IF NOT EXISTS idx_share_events_event_key ON share_events(event_key);
CREATE INDEX IF NOT EXISTS idx_share_events_share_event_key ON share_events(share_id, event_key);