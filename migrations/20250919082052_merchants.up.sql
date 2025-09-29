-- Add up migration script here
CREATE TABLE IF NOT EXISTS merchants (
  id         SERIAL PRIMARY KEY,
  account    VARCHAR NOT NULL UNIQUE,
  name       VARCHAR NOT NULL UNIQUE,
  apikey     VARCHAR NOT NULL UNIQUE,
  webhook    VARCHAR NOT NULL,
  eth        VARCHAR NOT NULL,
  updated_at TIMESTAMP NOT NULL
)
