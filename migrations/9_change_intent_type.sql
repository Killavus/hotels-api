DROP TABLE order_payments;
CREATE TABLE order_payments (
  id INTEGER PRIMARY KEY,
  payment_intent_id TEXT NOT NULL,
  order_id INTEGER NOT NULL,
  FOREIGN KEY(order_id) REFERENCES orders(id)
);

