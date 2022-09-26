CREATE TABLE rooms (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    beds INTEGER NOT NULL DEFAULT 1,
    pets_allowed BOOLEAN NOT NULL DEFAULT FALSE,
    price_in_cents INTEGER NOT NULL,
    hotel_id INTEGER NOT NULL,
    FOREIGN KEY(hotel_id) REFERENCES hotels(id)
);

