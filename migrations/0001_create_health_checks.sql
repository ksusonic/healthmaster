CREATE TABLE IF NOT EXISTS health_checks (
    timestamp DateTime,
    target String,
    url String,
    status UInt16,
    latency_ms UInt32,
    success UInt8,
    error String
) ENGINE = MergeTree
ORDER BY (target, timestamp);

