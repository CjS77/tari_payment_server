# Automated handling of Shopify manual payments

# Configuration
Configuration is done exclusively through environment variables.

The most important configuration variable is the one that carries the signing key for access tokens.

## Signing keys

When users authenticate via the `/auth` endpoint, they receive an access token that is signed with a secret key. This key
is defined in `TPG_JWT_SIGNING_KEY`. This must be a hexadecimal of 64 characters, and represent a valid Tari secret key.

You must also set the `TPG_JWT_VERIFICATION_KEY` variable, which corresponds to the public key of `TPG_JWT_SIGNING_KEY`.
We ask you to configure both to avoid fat finger errors. It's also it's easy to share and/or look up the public key if 
it is stored in the configuration, rather than having to calculate it from the secret all the time.

When the server starts, it will load the secret-public key pair from the environment and verify that the public key is
correct.

There are _some_ cases where you might not want to store the keys in environment variables (test servers, CI). In these cases, 
when the environment variables are not set, the server will generate a new random keypair and save the values to a temporary
file on the file system.

To indicate that this is generally a bad idea, you'll see the following messages in the logs:

```text
🚨️🚨️🚨️ 
The JWT signing key has not been set. I'm using a random value for this session.DO NOT operate on
production like this since you may lose access to data. 
🚨️🚨️🚨️

🚨️🚨️🚨️ 
The JWT signing key for this session was written to {filename}. If this is a production
instance, you are doing it wrong! 
Set the TPG_JWT_SIGNING_KEY and TPG_JWT_VERIFICATION_KEY environment variables instead. 
🚨️🚨️🚨️
```

## Shopify whitelisting

Orders are submitted to the server via the `/shopify/webhook/*` endpoints. This are called by shopify's webhook system.
However, if anyone could make calls to that endpoint, they could submit fake orders to the server. 
One set of protections against this is to whitelist the IP addresses of Shopify's webhook servers.

All endpoints under the `/shopify` scope are checked against the shopify IP whitelist. These are configured via the
`TPG_SHOPIFY_IP_WHITELIST` environment variable. This is a comma-separated list of IP addresses.
For example, 
```
TPG_SHOPIFY_IP_WHITELIST=192.168.1.1,192.168.1.5,10.0.0.2
```

When an incoming request is made, the server will check the IP address of the request against the whitelist. The IP is taken
from the remote peer of the connection. If the Tari payment server is behind a load balancer, this might cause the check
to fail, since the IP address of the load balancer will be checked, rather than the IP address of the Shopify server.

To work around this, you can set the `TPG_USE_X_FORWARDED_FOR` or `TPG_USE_FORWARDED` environment variables to `1` or `true`. 
The server will then use the IP address in the `X-Forwarded-For` or `Forwarded` headers, respectively.

Your proxy or load balancer must then be configured to set these headers and should take precautions against header spoofing.

🚨️🚨️🚨️ **WARNING** 🚨️🚨️🚨️

Attackers can trivially spoof `X-Forwarded-For` or `Forwarded` headers. So be careful if using these options and ensure that 
your proxy or load balancer takes precautions to detect spoofing (such as comparing against the remote peer's IP address).

## Server configuration

The server is configured via the following environment variables:

- `TPG_HOST`: This variable is used to set the host of the server. If not set, the default value is `127.0.0.1`.
- `TPG_PORT`: This variable is used to set the port on which the server will run. If not set or if the provided value is not a valid port, the default value is `8360`.
- `TPG_SHOPIFY_API_KEY`: This variable is used to set the API key for your Shopify app. If not set, an error message will be logged, and the value will be set to an empty string.
- `TPG_DATABASE_URL`: This variable is used to set the URL for the TPG database. If not set, an error message will be logged, and the value will be set to an empty string.
               It's of the form `sqlite://<path to database file>` or `postgres://<username>:<password>@<host>/<database>`.


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
