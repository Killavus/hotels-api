CREATE TABLE customers (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL,
    billing_street TEXT NOT NULL,
    billing_street_add TEXT NOT NULL DEFAULT "",
    billing_city TEXT NOT NULL,
    billing_postcode TEXT NOT NULL,
    billing_country TEXT NOT NULL
);
