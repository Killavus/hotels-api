# Hotels API w/Axum

A minimal example of booking.com-like API for ordering multiple rooms and paying for them with [Stripe](https://stripe.com/).

It uses [axum](https://github.com/tokio-rs/axum) web framework together with [sqlx](https://github.com/launchbadge/sqlx) to run SQL queries.

## Setup

This project uses `dotenv` under the hood, so take a look at `.env.example` to see all environment variables used to configure the project - then copy it to `.env` and configure. To compile this, you need to have `DATABASE_URL` set _at minimum_ due to how `sqlx` perform compile-time checking of your SQL queries.

You need to have [Rust toolchain](https://rustup.rs/) installed.

```
# be sure to have DATABASE_URL environment variable set either in .env file or in your shell!
cargo run --bin migrate
cargo run
```

## Endpoints

* `GET /rooms` - returns a list of existing rooms together with hotel information.
* `POST /order` - creates a new order. It expects following JSON body:

```
{
    "rooms_order": [{ room_id: number, start_date: "YYYY-MM-DD", end_date: "YYYY-MM-DD" }],
    "address_details": {
        "email": string,
        "billing_street": string,
        "billing_street_add": string (optional),
        "billing_postcode": string,
        "billing_city": string,
        "billing_country": string
    }
}
```

* `POST /order/:id/payment` - creates a [payment intent](https://stripe.com/docs/payments/payment-intents) for the order if not exists or returns existing one's `client_secret`.

## License

[Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0.txt)
