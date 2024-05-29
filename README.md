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
- `RUST_LOG`: This variable is used to set the log level for the server. If not set, the default value is `info`.
              For production, I recommend setting this to:
              `RUST_LOG="warn,tari_payment_server=info,tari_payment_engine=info,access_log=info"`
              At the minimum, set `access_log=INFO` to use the access log middleware to log all incoming requests.

## Access logs
Access logs are printed to the terminal, as long as the `RUST_LOG` environment variable is set to `info` or has the `access_log=INFO` term
included. It is straightforward to redirect these logs to a file or a log aggregator, to be ingested by your favourite log management system.

The log format is
 * Time when the request was started to process (`2024-05-29T08:42:20.029845041Z`)
 * Remote IP-address (IP-address of proxy if using reverse proxy) (`127.0.0.1`)
 * X-Forwarded-For header (`x-forwarded-for: 192.168.1.100`)
 * Forwarded header (`forwarded-for: 1.2.3.4`)
 * First line of request (`"POST /shopify/webhook/checkout_create HTTP/1.1"`)
 * Response status code (`200`)
 * User agent (`ua:"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3"`)
 * Authentication token (`auth:"eyJ..."`)
 * Access token (`access:"eyJ..."`)
 * Time taken to serve the request in milliseconds (`5.353228 ms`)

An example log output is
```text
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.029845041Z 127.0.0.1 x-forwarded-for: - forwarded: - "POST /shopify/webhook/checkout_create HTTP/1.1" 200 ua:"-" auth:"-" access:"-" 5.353228 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.183878425Z 127.0.0.1 x-forwarded-for: - forwarded: - "POST /auth HTTP/1.1" 400 ua:"-" auth:"some made up nonsense" access:"-" 0.142130 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.184103124Z 127.0.0.1 x-forwarded-for: - forwarded: for=192.168.1.100 POST /wallet/incoming_payment HTTP/1.1" 401 ua:"-" auth:"-" access:"-" 5.718148 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.215215236Z 127.0.0.1 x-forwarded-for: 192.168.1.100 forwarded: - POST /wallet/incoming_payment HTTP/1.1" 401 ua:"-" auth:"-" access:"-" 4.793100 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.215669395Z 127.0.0.1 x-forwarded-for: 1.2.3.4 forwarded: - "POST /wallet/incoming_payment HTTP/1.1" 401 ua:"-" auth:"-" access:"-" 9.104270 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.267856222Z 127.0.0.1 x-forwarded-for: - forwarded: - "POST /auth HTTP/1.1" 200 ua:"-" auth:"eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ.eyJhZGRyZXNzIjp7Im5ldHdvcmsiOiJtYWlubmV0IiwicHVibGljX2tleSI6ImI4OTcxNTk4YTg2NWIyNWI2NTA4ZDRiYTE1NGRiMjI4ZTA0NGYzNjdiZDlhMWVmNTBkZDQwNTFkYjQyYjYzMTQifSwibm9uY2UiOjEsImRlc2lyZWRfcm9sZXMiOlsidXNlciJdfQ.Uit7DJ2VtFdrDGiiDo4vQVKEc6TZ789hTbndrZXDR2QuAeQwlTzvnVPUBibJVwWRJTUFmy7n06amVC6HWhTcCw" access:"-" 7.055605 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.343505667Z 127.0.0.1 x-forwarded-for: - forwarded: - "POST /auth HTTP/1.1" 401 ua:"-" auth:"eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ.eyJhZGRyZXNzIjp7Im5ldHdvcmsiOiJtYWlubmV0IiwicHVibGljX2tleSI6ImI4OTcxNTk4YTg2NWIyNWI2NTA4ZDRiYTE1NGRiMjI4ZTA0NGYzNjdiZDlhMWVmNTBkZDQwNTFkYjQyYjYzMTQifSwibm9uY2UiOjEsImRlc2lyZWRfcm9sZXMiOlsidXNlciJdfQ.nMLtM8Cm-uNXdeo_XLXSX0Iqos_TV1F_uhty6I8X8GNthJMBhE2scU_V8HR2ZMYM4kFXdQTiXBplUe-TNLGTDg" access:"eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ.eyJleHAiOjE3MTcwNTg1NDAsImlhdCI6MTcxNjk3MjE0MCwiYWRkcmVzcyI6eyJuZXR3b3JrIjoibWFpbm5ldCIsInB1YmxpY19rZXkiOiJiODk3MTU5OGE4NjViMjViNjUwOGQ0YmExNTRkYjIyOGUwNDRmMzY3YmQ5YTFlZjUwZGQ0MDUxZGI0MmI2MzE0In0sInJvbGVzIjpbInVzZXIiXX0.oB8FdVJ6KS377SNnuYqX0E2AWRgTINyjST6tJuRfpkrOYe0mbLDZ927oRTlkkIUDyw4PY85Jlepamn6WF5_CDQ" 5.544898 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.388926978Z 127.0.0.1 x-forwarded-for: - forwarded: - "GET /api/unfulfilled_orders HTTP/1.1" 200 ua:"-" auth:"-" access:"eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ.eyJleHAiOjE3MTcwNTg1NDAsImlhdCI6MTcxNjk3MjE0MCwiYWRkcmVzcyI6eyJuZXR3b3JrIjoibWFpbm5ldCIsInB1YmxpY19rZXkiOiJiODk3MTU5OGE4NjViMjViNjUwOGQ0YmExNTRkYjIyOGUwNDRmMzY3YmQ5YTFlZjUwZGQ0MDUxZGI0MmI2MzE0In0sInJvbGVzIjpbInVzZXIiXX0.zrRjS_AeysZswQ3a5FhggXz8jAEZ2XGSwdu-Qfb9KVx9NvKfVZCVXOW8AyyJA4idcBr_N_wt1LYPchS0HghMBQ" 5.185608 ms
[2024-05-29T08:42:21Z INFO  access_log] 2024-05-29T08:42:21.253659261Z 127.0.0.1 x-forwarded-for: - forwarded: - "GET /api/search/orders?customer_id=bob&since=2024-03-11T0:0:0Z HTTP/1.1" 200 ua:"-" auth:"-" access:"eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ.eyJleHAiOjE3MTcwNTg1NDAsImlhdCI6MTcxNjk3MjE0MCwiYWRkcmVzcyI6eyJuZXR3b3JrIjoibWFpbm5ldCIsInB1YmxpY19rZXkiOiJhYTNjMDc2MTUyYzFhZTQ0YWU4NjU4NWVlYmExZDM0OGJhZGI4NDVkMWNhYjVlZjEyZGI5OGZhZmI0ZmVhNTVkIn0sInJvbGVzIjpbInJlYWRfYWxsIl19.kmWQe-PCmwi-_lNjw4sS132YQ8ly_Xx5hgkKooysc3M79lXbTfv-q4hViSBi9lEEiLuKeLc4hLHS223X_QT5CQ" 9.396370 ms
```

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
