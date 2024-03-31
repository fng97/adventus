-- create metrics table
CREATE TABLE metrics (
    id SERIAL PRIMARY KEY,
    metric_name VARCHAR(255) UNIQUE NOT NULL,
    count BIGINT DEFAULT 0 NOT NULL
);