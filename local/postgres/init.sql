CREATE TABLE IF NOT EXISTS messages (
    id SERIAL PRIMARY KEY,
    topic VARCHAR(255) NOT NULL,
    part INT NOT NULL,
    kafkaoffset BIGINT NOT NULL,
    payload TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_messages_topic ON messages(topic);
CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at);

-- New table for published data
CREATE TABLE IF NOT EXISTS published_data (
    id SERIAL PRIMARY KEY,
    topic VARCHAR(255) NOT NULL,
    part INT NOT NULL,
    kafkaoffset BIGINT NOT NULL,
    data TEXT NOT NULL,
    metadata TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_published_data_created_at ON published_data(created_at);
