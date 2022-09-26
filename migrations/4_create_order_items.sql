CREATE TABLE order_items (
    id INTEGER PRIMARY KEY,
    FOREIGN KEY(order_id) REFERENCES orders(id),
    FOREIGN KEY(room_id) REFERENCES rooms(id),
)
