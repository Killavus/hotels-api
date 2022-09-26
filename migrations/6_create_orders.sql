CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    stripe_intent_id TEXT NOT NULL,
    customer_id INTEGER NOT NULL,
    FOREIGN KEY(customer_id) REFERENCES customers(id)
);
