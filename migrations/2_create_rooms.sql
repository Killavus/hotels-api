CREATE TABLE rooms (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    beds INTEGER NOT NULL DEFAULT 1,
    pets_allowed INTEGER DEFAULT 0,
    price_in_cents INTEGER NOT NULL,
    FOREIGN KEY(hotel_id) REFERENCES hotels(id),
);

CREATE UNIQUE INDEX IF NOT EXISTS room_name_uniq ON rooms(hotel_id, name);
