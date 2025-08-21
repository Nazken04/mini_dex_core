CREATE TABLE trades (
    id UUID PRIMARY KEY,
    maker_order_id UUID NOT NULL,
    taker_order_id UUID NOT NULL,
    price DECIMAL NOT NULL,
    quantity DECIMAL NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL
);