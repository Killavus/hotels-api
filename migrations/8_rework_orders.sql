DELETE FROM order_items WHERE 1=1;
DELETE FROM orders WHERE 1=1;
DELETE FROM customers WHERE 1=1;

DROP TABLE orders;

CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    customer_id INTEGER NOT NULL,
    FOREIGN KEY(customer_id) REFERENCES customers(id)
);

CREATE TABLE order_payments (
  id INTEGER PRIMARY KEY,
  payment_intent_id INTEGER NOT NULL,
  order_id INTEGER NOT NULL,
  FOREIGN KEY(order_id) REFERENCES orders(id)
);
