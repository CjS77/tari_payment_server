# Tari tools

A Swiss army knife of utilities and tools that are useful for interacting with the Tari Payment Server.

## General CLI commands

Usage: `taritools [OPTIONS] <COMMAND>`

### address  

Generate and print a new random secret key and print the associated public key, Tari address, and emoji id

#### Example
    $ taritools address -n mainnet

```text
----------------------------- Tari Address -----------------------------
Network: mainnet
Secret key: fa04e1f0e5c05cfb43b7c97c1f8886d6ee34469ef830f0bdb271784857cd6a02
Public key: ead228cf7c6d80e0afb5ece75e8b1073084abfe8f9b904e75ad99d7ac5b63250
Address: ead228cf7c6d80e0afb5ece75e8b1073084abfe8f9b904e75ad99d7ac5b63250d8
Emoji ID: 😻🔎🍞📷🐞🐊🐩🗽💊💔🚀😱🎽🐶🌹🐓🌞🎤💯😷🚨💣🌋😱🎹🔩👙🐜💼💕🍯🎬🔨
------------------------------------------------------------------------
```

### token    

Generate a JWT token for use in authenticating with the Tari Payment Server (e.g. for Curl or Postman). 

Usually, it's much easier to use the interactive mode of Taritools and let it handle authentication for you.

When using this command, you can specify the roles desired for the login token (assuming they've been granted on the server of course).

Usage: ```taritools token [OPTIONS] --seckey <SECRET>```

Options:
* `-s`, `--seckey <SECRET>`. The secret key to use for the token
* `-n`, `--network <NETWORK>`. The network to use (nextnet, stagenet, mainnet). Default is mainnet
* `-r`, `--roles <ROLES>`. Roles you want the token to grant `[default: user]`

#### Example

    $ taritools token -s fa04e1f0e5c05cfb43b7c97c1f8886d6ee34469ef830f0bdb271784857cd6a02 -r user,read_all`

```text
----------------------------- Access Token -----------------------------
address: 😻🔎🍞📷🐞🐊🐩🗽💊💔🚀😱🎽🐶🌹🐓🌞🎤💯😷🚨💣🌋😱🎹🔩👙🐜💼💕🍯🎬🔨
address: ead228cf7c6d80e0afb5ece75e8b1073084abfe8f9b904e75ad99d7ac5b63250d8
network: mainnet
nonce: 1723150450
roles: user,read_all
token:
eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ.eyJhZGRyZXNzIjp7Im5ldHdvcmsiOiJtYWlubmV0IiwicHVibGljX2tleSI6ImVhZDIyOGNmN2M2ZDgwZTBhZmI1ZWNlNzVlOGIxMDczMDg0YWJmZThmOWI5MDRlNzVhZDk5ZDdhYzViNjMyNTAifSwibm9uY2UiOjE3MjMxNTA0NTAsImRlc2lyZWRfcm9sZXMiOlsidXNlcixyZWFkX2FsbCJdfQ.BFKjGQGVsEQD_6KP03W7zs2w88xvyEjRJoZVbyEvEX3W2HdQq_VsHcd0q_3KCetpb5HzzZaAgqD7Fk20LoYuCA
------------------------------------------------------------------------
```

### memo

Generate a memo signature in order to claim orders in storefronts

This is useful to paste into the memo field of a payment from the console wallet, but it's generally far more convenient 
to use the 'Claim order' command in the interactive mode of Taritools.

Usage: `taritools memo [OPTIONS] --seckey <SECRET> --order <ORDER_ID>`

Options:
* `-s`, `--seckey <SECRET>`. The user's wallet secret key
* `-n`, `--network <NETWORK>`. The network to use (testnet, stagenet, mainnet). Default is mainnet
* `-o`, `--order <ORDER_ID>`. The order number associated with this payment. Generally extracted from the memo

#### Example

    $ taritools memo -s fa04e1f0e5c05cfb43b7c97c1f8886d6ee34469ef830f0bdb271784857cd6a02 -o 12345

```text
----------------------------- Memo Signature -----------------------------
Wallet address: ead228cf7c6d80e0afb5ece75e8b1073084abfe8f9b904e75ad99d7ac5b63250d8
Public key    : ead228cf7c6d80e0afb5ece75e8b1073084abfe8f9b904e75ad99d7ac5b63250
emoji id      : 😻🔎🍞📷🐞🐊🐩🗽💊💔🚀😱🎽🐶🌹🐓🌞🎤💯😷🚨💣🌋😱🎹🔩👙🐜💼💕🍯🎬🔨
Secret        : fa04e1f0e5c05cfb43b7c97c1f8886d6ee34469ef830f0bdb271784857cd6a02
Network       : mainnet
auth: {
  "address":"ead228cf7c6d80e0afb5ece75e8b1073084abfe8f9b904e75ad99d7ac5b63250d8",
  "order_id":"123456",
 "signature":"402e285311af310679abe8bd60e6ee94143417d95fa2a28e7a1306f4ac72651b51e485962c229af1c88cb9e161678e68498dc40f15c37924cea19873114a4b01"
}
------------------------------------------------------------------------
```

### payment

Generate a payment authorization signature to acknowledge a payment to a hot wallet.

This command will very seldom be used directly outside of testing.

Usage: `taritools payment [OPTIONS] --seckey <SECRET> --sender <SENDER>`

Options:
* `-s`, `--seckey <SECRET>`. The payment wallet's secret key
* `-n`, `--network <NETWORK>`. The network to use (testnet, stagenet, mainnet). Default is mainnet
* `-c`, `--nonce <NONCE>`. A monotonically increasing nonce. The current Unix epoch is a good stateless means of generating a nonce, assuming the calls aren't made more than once per second. Default is 1
* `-a`, `--amount <AMOUNT>`. The amount of the payment, in Tari. Default is 250
* `-t`, `--txid <TXID>`. The transaction identifier. Typically, the kernel signature in Tari. Default is payment001
* `-m`, `--memo <MEMO>`. The memo attached to the transfer
* `-o`, `--order <ORDER_ID>`. The order number associated with this payment. Generally extracted from the memo
* `-x`, `--sender <SENDER>`. The sender's address

### confirm  

Generate a transaction confirmation signature to confirm a payment to a hot wallet.
This command will very seldom be used directly outside of testing.

The arguments permissible are a subset of the `payment` command arguments:
* `-s`, `--seckey <SECRET>`. The payment wallet's secret key
* `-n`, `--network <NETWORK>`. The network to use (testnet, stagenet, mainnet). Default is mainnet
* `-c`, `--nonce <NONCE>`. A monotonically increasing nonce. The current Unix epoch is a good stateless means of generating a nonce, assuming the calls aren't made more than once per second. Default is 1
* `-t`, `--txid <TXID>`. The transaction identifier. Typically, the kernel signature in Tari. Default is payment001

## Shopify tools

There are several subcommands for interacting with your shopify store. Your environment (or `.env` file)
must contain the following variables:
* `TPG_SHOPIFY_SHOP`: Your Shopify shop name, e.g. `my-shop.myshopify.com`
* `TPG_SHOPIFY_API_VERSION`: Optional. The API version to use. Default is `2024-04`.
* `TPG_SHOPIFY_STOREFRONT_ACCESS_TOKEN`: Your Shopify storefront access token. e.g. `yyyyyyyyy`
* `TPG_SHOPIFY_ADMIN_ACCESS_TOKEN`: Your Shopify admin access token. e.g. `shpat_xxxxxxxx`
* `TPG_SHOPIFY_API_SECRET`: Your Shopify API secret. e.g. `aaaaaaaaaaaa`

Not all of these environment variables are required for all commands, but the `TPG_SHOPIFY_ADMIN_ACCESS_TOKEN` 
 and `TPG_SHOPIFY_SHOP` are required for most of the important administrative commands in taritools.

### shopify webhooks

As part of the configuration process and integration with Shopify, you will need to set up shopify webhooks.

You can list and install the webhooks using the `taritools shopify webhooks install|list` commands.

Further details are given in the [Shopify integration guide](../SHOPIFY_INTEGRATION.md).

### shopify orders

Retrieve or modify Shopify orders. You should very rarely use these commands (except, perhaps, `get`) 
directly outside a testing environment, since they can cause the Tari Payment server and the storefront 
to get out of sync.

Commands:
* `get  <ID>`:     Fetch the order with the given ID
* `cancel <ID>`:  Cancel the order with the given ID
* `pay <ID> <AMOUNT> <CURRENCY>`:  Mark the given order as paid on Shopify. This does not facilitate any transfer of funds; it only tells 
Shopify that the order has been paid for.

### wallet   

Commands created for use by console wallet to communicate with the Tari Payment Server. You do not need to use these 
commands directly.
 
## Interactive mode

You can also run taritools in interactive mode by running `taritools` without any arguments. This will present you with 
a menu of options to choose from. 

This is the recommended way to use `taritools`, as it presents an easy-to-use menu based system, automatic authentication,
and helpful feedback and results.

A short summary of all the commands is given below:

You can start typing any part of a command to filter the list. For example, typing `order` will show all commands that
contain the word `order`.

You can also use the arrow keys to navigate the menu, and the `Enter` key to select a command.

### Commands Table

| Command                  | Scope | Description                                                                                                                                    |
|--------------------------|-------|------------------------------------------------------------------------------------------------------------------------------------------------|
| Add profile              | User  | Add a new profile to the config file, simplifying authentication for other commands.                                                           |
| Claim Order              | User  | Claim an order for the user, associating it with the user's wallet address.                                                                    |
| List payment addresses   | User  | List the hot wallet payment addresses, including a QR code to easily scan the address into say, Aurora.                                        |
| My Account               | User  | View the user's account balance and order summary.                                                                                             |
| Account History          | User  | View the user's detailed account history, including associated wallet <br/><br/>addresses, order history and payments.                         |
| My Open Orders           | User  | View the user's open orders, showing all orders that are currently active and not yet completed.                                               |
| My Orders                | User  | View the user's orders, providing a comprehensive list of all orders placed by the user.                                                       |
| My Payments              | User  | Displays all payments made by the wallet configured in the profile.                                                                      <br/> |
| Add authorized wallet    | Admin | Add a new authorized hot wallet to the server. Requires Super-Admin privileges                                                                 |
| Cancel Order             | Admin | Cancel an existing order.                                                                                                                      |
| Edit memo                | Admin | Edit the memo of an order, allowing changes to the notes or comments associated with the order.                                                |
| Fetch Tari price         | Admin | Fetch the current Tari price as used by the server.                                                                                            |
| History for Account Id   | Admin | Show history for a specific account ID, displaying all transactions and activities associated with that account.                               |
| History for Address      | Admin | Show history for a specific wallet address, listing all transactions and activities linked to that address.                                    |
| Issue Credit             | Admin | Issue a credit for a customer id. Used for providing a refund or credit to the user's account for a specific order.                            |
| List authorized wallets  | Admin | List all authorized hot wallet addresses.                                                                                                      |
| Mark order as Paid       | Admin | Mark an order as paid, updating the order status to reflect that payment has been received.                                                    |
| Order by Id              | Admin | Find an order by its (Storefront) ID.                                                                                                          |
| Orders for Address       | Admin | List orders for a specific wallet address                                                                                                      |
| Payments for Address     | Admin | List payments for a specific wallet address                                                                                                    |
| Reassign Order           | Admin | Reassign an order to a different customer id.                                                                                                  |
| Remove authorized wallet | Admin | Remove an authorized hot wallet address. This does not affect the wallet itself.                                                               |
| Reset Order              | Admin | Reset an order status, clearing its current (expired) status.                                                                                  |
| Server health            | Admin | Check the server health.                                                                                                                       |
| Set Tari price           | Admin | Set the Tari price, updating the exchange rate for the cryptocurrency. This will also update the price of EVERY product in the store.          |

### Navigation Commands

| Command    | Description                                                                                                |
|------------|------------------------------------------------------------------------------------------------------------|
| Exit       | Exit the application, closing the program and ending the current session.                                  |
| Logout     | Logout from the application, ending the user's session and requiring re-authentication for further access. |
| Back       | Go back to the previous menu, returning to the last screen or menu option.                                 |
| Admin Menu | Open the admin menu, providing access to administrative commands and settings.                             |
| User Menu  | Open the user menu, providing access to user-specific commands and settings.                               |
