# Installation Guide

This guide provides a walkthrough of installing and setting up the Tari Payment Gateway with:
* A Shopify storefront
* A Tari console wallet.

The Tari Payment Server consists of the following parts:

* The **Tari Payment Server** (TPS), which is a REST API server in front of the Tari Payment Engine and a Database,
  that
  tracks orders, payments, and the links between them. It has support for automatic account tracking (for example,
  if a Tari wallet sends more Tari than is needed, TPS will remember which store accounts were used with that
  address and apply the excess as a credit in future orders).
* A Storefront. As of today, only **Shopify Storefront**s are supported, but integrations for other popular
  storefronts can be added.
* A **hot wallet**, which acts as the receiver for all payments for orders.

The hot wallet, TPS and storefront communicate with each other via webhooks and REST APIs. We employ a stateless
communication protocol, which requires that all messages be signed. Rigging this up accounts for the majority 
of the complexity of server setup.

This guide will walk you through everything you need to do to set up a Tari Payment Server. If anything is unclear,
or you have a suggestion for improving this document, please submit an issue or a pull request.
            
# Install the Tari Payment Server

_This section provides information for a SysAdmin or DevOps engineer to aid in installing the Tari Payment Server._

_If you want to run TPS in Kubernetes, there are also docker images of the Tari Payment Server available 
on [ghcr]_.

[ghcr]: https://github.com/CjS77/tari_payment_server/pkgs/container/tari_payment_server "Github container repository"

[Tari payment server] is a REST API server that tracks orders, payments, and the links between them.

It uses a database backend (only SQLite is supported at the moment) to store order and payment information.

## Prerequisites and dependencies

On a Gnu/Linux system, you will need to install the following dependencies if you _building from source_:

* build-essential 
* ca-certificates 
* gcc 
* libssl-dev 
* pkg-config 
* libsqlite3-dev (if using SQLite) 
* libpq-dev 
* unzip (if download the binaries from Github)

**Do:** You can use this bash command to install the dependencies:

```
sudo apt-get update && apt-get install -y build-essential ca-certificates gcc libssl-dev pkg-config libsqlite3-dev libpq-dev unzip
```

**Do:** If you are simply using binaries, you should not need to install any additional dependencies.

## The server binary

You can [build from source](README.md#building-from-source),
or download a binary from the [releases page](https://github.com/CjS77/tari_payment_server/tags).

Make sure that both `tari_payment_server` and `taritools` are in your `PATH`.

[Tari payment server]: https://github.com/CjS77/tari_payment_server "Tari Payment Server on Github"
     
## Server configuration
Tari Payment server is configured using environment variables. You can set the bulk of these in a `.env` file in the 
root of the same directory as the server binary. However, _we strongly recommend_ using a vault or secret manager to
store the secret keys and other sensitive information and set the environment variables in the shell to avoid
having secrets stored on disk.

**Do:** Copy the `.env.example` file to `.env` and edit the values to match your setup.

Storefront environment variables are explained in the relevant storefront integrations guides.

* For Shopify, see [Shopify Integration](./SHOPIFY_INTEGRATION.md)
       
**Do:** You will also want to configure the following environment variables:

- `TPG_HOST`: This variable is used to set the host of the server. If not set, the default value is `127.0.0.1`.
- `TPG_PORT`: This variable is used to set the port on which the server will run. 
   If not set or if the provided value is not a valid port, the default value is `8360`.
- `RUST_LOG`. Sets the verbosity of the log messages. A good default that provides plenty of useful information 
  without generating information overload is 
  ```
    RUST_LOG=warn,access_log=info,tari_payment_server=info,tari_payment_engine=info,tpg_common=info,e2e_tests=info,sqlx=warn,shopify_tools=info
  ```
  At the minimum, set `access_log=INFO` to use the access log middleware to log all incoming requests.
- `TPG_DATABASE_URL`: This variable is used to set the URL for the TPG database. If not set, an error message will be 
  logged, and the value will be set to an empty string.
  It's of the form `sqlite://<path to database file>` or `postgres://<username>:<password>@<host>/<database>`.
- `TPG_STRICT_MODE`: Enable strict mode. When `1` or `true`, _only_ the order_id field will be used to identify orders.

**Do:** Set `TPG_PAYMENT_WALLET_ADDRESS` to the public key of the wallet that will receive payments. 
This key must be present in the Authorized Wallet list in the database. (Note: This envar will be deprecated in future)

### Forwarding remote IP addresses
                        
To use the `X-Forwarded-For` or `Forwarded` headers to get the remote IP address, set the following environment variables:
- TPG_USE_X_FORWARDED_FOR=1
- TPG_USE_FORWARDED=1

Unless you're behind a reverse proxy, you should not need set these variables.               

See the [section below](#storefront-whitelisting) for more information on how to use these variables.

### Order expiry times

During the normal course of events, there will be many abandoned orders accumulating in the system. To prevent this 
becoming a long-term drain of performance and resources, unclaimed and unpaid order are set to expire after a certain
period.

Unclaimed orders, which are a minor vector for a Sybil or DoS attack should be set to expire after a relatively short
period. The default is 2 hours.

`TPG_UNCLAIMED_ORDER_TIMEOUT=2 # Expiry time for new, unclaimed orders, in hours`

Orders that _have been claimed_ and are therefore associated with a wallet address have a longer default timeout of 48
hours.

`TPG_UNPAID_ORDER_TIMEOUT=48 # Expiry time for unpaid orders, in hours`
      
## Execution permissions

**Do:** Execution permissions get stripped when uploading the artifacts to Github. Run
```bash
chmod +x tari_payment_server taritools
```
to restore the permissions.

## Signing keys

**This is the most important configuration step by some distance. Read this section carefully.** 

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

### Generating a new keypair

[taritools] has a command to generate a new keypair.

```bash
taritools address
```

This will generate a new keypair and print the public key to the terminal. 
You can then set the `TPG_JWT_SIGNING_KEY` and `TPG_JWT_VERIFICATION_KEY` environment variables to the values that 
you've just generated.

## Storefront whitelisting

You can specify a whitelist of IP addresses that are allowed to send webhook requests to the server. 

**Do:** For shopify, leave this list empty, since Shopify requests come from different IP addresses and they 
can change frequently.

**Do:** To secure the shopify webhook endpoints, use 
[HMAC signatures](./SHOPIFY_INTEGRATION.md#configure-webhooks-to-interact-with-your-server) instead.
  
Other storefronts that don't make use of HMAC signatures should have their IP addresses added to the whitelist. 
When an incoming request is made, the server will check the IP address of the request against the whitelist. The IP is taken
from the remote peer of the connection. If the Tari payment server is behind a load balancer, this might cause the check
to fail, since the IP address of the load balancer will be checked, rather than the IP address of the Shopify server.

To work around this, you can set the `TPG_USE_X_FORWARDED_FOR` or `TPG_USE_FORWARDED` environment variables to `1` or `true`.
The server will then use the IP address in the `X-Forwarded-For` or `Forwarded` headers, respectively.

Your proxy or load balancer must then be configured to set these headers and should take precautions against header spoofing.

🚨️🚨️🚨️ **WARNING** 🚨️🚨️🚨️

Attackers can trivially spoof `X-Forwarded-For` or `Forwarded` headers. So be careful if using these options and ensure that
your proxy or load balancer takes precautions to detect spoofing (such as comparing against the remote peer's IP address).

## Access logs
Access logs are printed to the terminal, as long as the `RUST_LOG` environment variable is set to `info` or has the 
`access_log=INFO` term included. 
It is straightforward to redirect these logs to a file or a log aggregator, to be ingested by your favourite log 
management system.

The log format is
* Time when the request was started to process (`2024-05-29T08:42:20.029845041Z`)
* Remote IP-address (IP-address of proxy if using reverse proxy) (`127.0.0.1`)
* X-Forwarded-For header (`x-forwarded-for: 192.168.1.100`)
* Forwarded header (`forwarded-for: 1.2.3.4`)
* First line of request (`"POST /shopify/webhook/checkout_create HTTP/1.1"`)
* Response status code (`200`)
* User agent (`ua:"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3"`)
* Time taken to serve the request in milliseconds (`5.353228 ms`)

An example log output is
```text
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.029845041Z 127.0.0.1 x-forwarded-for: - forwarded: - "POST /shopify/webhook/checkout_create HTTP/1.1" 200 ua:"-" 5.353228 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.183878425Z 127.0.0.1 x-forwarded-for: - forwarded: - "POST /auth HTTP/1.1" 400 ua:"-"" 0.142130 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.184103124Z 127.0.0.1 x-forwarded-for: - forwarded: for=192.168.1.100 POST /wallet/incoming_payment HTTP/1.1" 401 ua:"-"" 5.718148 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.215215236Z 127.0.0.1 x-forwarded-for: 192.168.1.100 forwarded: - POST /wallet/incoming_payment HTTP/1.1" 401 ua:"-"" 4.793100 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.215669395Z 127.0.0.1 x-forwarded-for: 1.2.3.4 forwarded: - "POST /wallet/incoming_payment HTTP/1.1" 401 ua:"-"" 9.104270 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.267856222Z 127.0.0.1 x-forwarded-for: - forwarded: - "POST /auth HTTP/1.1" 200 ua:"-"" 7.055605 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.343505667Z 127.0.0.1 x-forwarded-for: - forwarded: - "POST /auth HTTP/1.1" 401 ua:"-"" 5.544898 ms
[2024-05-29T08:42:20Z INFO  access_log] 2024-05-29T08:42:20.388926978Z 127.0.0.1 x-forwarded-for: - forwarded: - "GET /api/unfulfilled_orders HTTP/1.1" 200 ua:"-"" 5.185608 ms
[2024-05-29T08:42:21Z INFO  access_log] 2024-05-29T08:42:21.253659261Z 127.0.0.1 x-forwarded-for: - forwarded: - "GET /api/search/orders?customer_id=bob&since=2024-03-11T0:0:0Z HTTP/1.1" 200 ua:"-"" 9.396370 ms
```
           
## Configuring the database

`taritools` contains an embedded instance of `sqlx` and all of the database migrations, so setting up the database is
as simple as.

```bash
taritools setup migrate
```

The DB administrator _may_ also keep a copy of the migration files on the server 
(`tari_payment_engine/src/sqlite/migrations/*`) and override the default migration set by providing the path to the
migration files.

```bash
taritools setup migrate --path /path/to/migrations
```

## Assigning the Super Admin role

The server administrator(s) do not need direct access to the server; they can access admin functions via the REST
server. However, the `SuperAdmin` role needs to be configured directly in the database to bootstrap the system.

To set up the `SuperAdmin` role, you need the Tari address of the Super Admin user. You can use the `taritools
address` command to create a new address if needed.

The Super Admin must keep their secret key secure, since the Super Admin has complete control over the payment server.

```bash
taritools setup add-user -a <tari-address-of-super-admin> -r super_admin
```

               
## Setup complete!

You can now run the server with the following command:

```bash
tari_payment_server
```

The rest of the server management can be done by the Super Admin user via the REST API.
   
# Super Administrator configuration

All the steps in this section are done by the Super Admin user via the REST API. For these steps to run successfully,
the server must be configured properly, and running as described in [Server configuration](#server-configuration).

All the actions described in this section are executed via the REST API using the `taritools` CLI utility.
To have `taritools` run properly, you must
* set up your `.env` file,
* create a super-admin profile

## Add an admin profile

Create or edit `~/.taritools/config.toml` and add a profile for the Super Admin user. The profile should look something

```toml
[[profiles]]
name = "SuperAdmin"
address = "super-admin-tari-address"
secret_key = "super-admin-secret-key"
roles = ["super_admin"]
server = "http://172.17.0.2:4444"
```

If you don't want to store the secret key on disk, you can set the `secret_key_env` field to the name of an environment
variable that contains the secret key, and populate that environment variable from a vault.

```toml
[[profiles]]
name = "More secure SuperAdmin"
address = "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt"
secret_key_envar = "TEST_KEY_SECRET"
roles = ["super_admin"]
server = "http://172.17.0.2:4444"
```

## Add an authorised wallet

1. Run `taritools`
2. Select `Admin | Add authorized wallet`


# Tari console hot wallet

You should use the Tari console wallet as your hot wallet for the payment server.

## Install the Tari console wallet.

Follow the instructions on the [Tari Website](https://tari.com/downloads) to install and configure the Tari console
wallet.

Tari Payment Server takes advantage of the `notifier` script to let the server know when payments are received and
confirmed.

## Configure the notifier script.

1. Edit the Tari configuration file. This is usually located at `$HOME/.tari/{network}/config/config.toml`. Replace 
   `{nework}` with the network you are using, e.g. `mainnet`.
2. Under the `[wallet]` section, add or edit the following line:
   ```toml
   notify_file = "{path_to_HOME}/.taritools/tps_notify.sh"
   ```
3. Add a wallet profile to your taritools configuration file.
   1. Edit `$HOME/.taritools/config.toml`.
   2. Add a profile with the name `TPS Hot Wallet`. Something like:
     ```toml
     [[profiles]]
     name="TPS Hot Wallet"
     address="14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt"
     # For security reasons, we suggest you don't store the secret key in the config file.
     secret_key=""
     # The secret key will be loaded from the specified enviroment variable instead.
     secret_key_env="TPG_HOT_WALLET_SECRET_KEY"
     roles=["user"]
     server="https://my_tps_server"
     ```
4. Restart your hot wallet, and you should be good to go. Watch the logs in the TPS to check that the wallet hits
   the `/wallet/incoming_payment` and `/wallet/tx_confirmation` endpoints.

## Set the Tari price

For storefronts that don't allow the use of custom currencies, including Shopify, you need to set the Tari Price.

1. Run taritools.
2. Select `Admin` | `Set Tari Price`.
3. Enter the price (Tari per USD).
4. Confirm the price.

If you have a lot of products in your store, it may take a while to propagate the new price to the storefront server.



# Tari tools

## Configuring `taritools`
The `taritools` CLI utility is configured using the same `.env` file as the Tari Payment Server.
