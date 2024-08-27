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

You can use this bash command to install the dependencies:

```
sudo apt-get update && apt-get install -y build-essential ca-certificates gcc libssl-dev pkg-config libsqlite3-dev libpq-dev
```

If you are simply using binaries, the only dependencies are:

* libsqlite3
* ca-certificates
* libssl3

```
sudo apt-get update && apt-get install -y ca-certificates libssl3 libsqlite3
```
  
## The server binary

You can build from source,
```bash
cargo build --release --features=sqlite
```

or download a binary from the [releases page](https://github.com/CjS77/tari_payment_server/tags).

Make sure that both `tari_payment_server` and `taritools` are in your `PATH`.

[Tari payment server]: https://github.com/CjS77/tari_payment_server "Tari Payment Server on Github"


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
     address="b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d"
     # For security reasons, we suggest you don't store the secret key in the config file.
     secret_key=""
     # The secret key will be loaded from the specified enviroment variable instead.
     secret_key_env="TPG_HOT_WALLET_SECRET_KEY"
     roles=["user"]
     server="https://my_tps_server"
     ```
4. Restart your hot wallet, and you should be good to go. Watch the logs in the TPS to check that the wallet hits
   the `/wallet/incoming_payment` and `/wallet/tx_confirmation` endpoints.

# Tari tools

## Configuring `taritools`
The `taritools` CLI utility is configured using the same `.env` file as the Tari Payment Server.
