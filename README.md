# Automated handling of Shopify manual payments

# Building from source

## Configuring the test database

If you run `cargo build` and receive errors from `sqlx` along the lines of

```text
error: error returned from database: (code: 1) no such table: orders
  --> src/db/sqlite/orders.rs:20:5
   |
20 | /     sqlx::query!(
21 | |         r#"
22 | |             INSERT INTO orders (
23 | |                 order_id,
...  |
36 | |         timestamp,
37 | |     )
   | |_____^

```

Then you must run the migrations first. This can be done by

1. Copy `.env.sample` to `.env`
2. Edit `.env` to set the `DATABASE_URL` to a valid sqlite database path, or use the defaults as-is.
3. Run `./scripts/migrate.sh` to set up the database. You'll see some output like
    ```text
    ./scripts/migrations.sh 
    Applied 1/migrate create orders (1.121853ms)
    Applied 2/migrate create payments (854.985µs)
    ...
    Ok
    ```
4. Run `cargo build` again.
