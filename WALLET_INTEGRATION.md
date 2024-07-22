# Integrating wallets with Tari Payment Server

## General requirements

There are two things that need to happen for users to successfully complete purchases using Tari Payment Server:

1. A payment for at least the amount of the order needs to be made to an authorised hot wallet. The hot wallet does 
   not need to live on the same machine as the payment server, but it must be registered and whitelisted with the
   payment server.
2. We need to link the order number for the purchase to a Tari payment address with a signed message.

This can be done in one, or two steps.

## Single step - Provide claim message in memo field.

In the usual purchase flow of

``` Finalise order on storefront --> Pay with Tari --> Claim order ```

the user knows the order number at the time of payment. 
A signed claim message can be provided in the `memo` field of the payment transaction. and this will be forwarded 
and verified by Tari Payment Server.

This requires some work on the part of the wallet developer, since generating a claim message needs to be done in 
the wallet transaction process. Generating a claim message is straightforward but not trivial.

The authoritative signature methodology is in the [MemoSignature] method of the `tari_payment_engine` source code.
The code takes priority over any deviations between this document and the code.

When users claim an order, we cannot just believe what users say,
because this would let folks attach other wallet addresses to their orders and hope that one day someone makes a payment
from that wallet and their order will then be fulfilled.

Users need to _prove_ that they own the wallet address they provide in the order. This is done by signing a message
with the wallet's private key. The message is constructed from the wallet address and the order ID (preventing
naughty people from using the same signature for their own orders, and again, trying to get free stuff).

The signature is then stored in the order memo field, and the payment server can verify the signature by checking
the wallet's public key against the signature.

### Message format
The message is constructed by concatenating the wallet address and the order ID, separated by a colon.
The challenge is a domain-separated Schnorr signature. The full format is:
```text
   {aaaaaaaa}MemoSignature.v1.challenge{bbbbbbbb}{address}:{order_id}
```
where
  * `aaaaaaaa` is the length of `MemoSignature.v1.challenge`, i.e. 25 in little-endian format.
  * `bbbbbbbb` is the length of `address`(64) + `:`(1) + `order_id.len()` in little-endian format.
  * `address` is the Tari address of the wallet owner, in hexadecimal
  * `order_id` is the order ID, a string

The message is then hashed with `Blake2b<U64>` to get the challenge.

### Creating the signature

You **should not** implement the above algorithm manually (it is provided here for reference). Instead, use the
`MemoSignature::create` method in the `tari_payment_engine` crate. This method produces a signature that can be 
serialized into JSON and stored in the memo field of the transaction.

A valid memo that contains an order claim has the following requirements:
1. The entire memo must be a single JSON object.
2. The JSON object must a `claim` key, with the claim object attached.
3. The claim object must have the following format
   * `address`: The Tari address of the wallet owner, in hexadecimal.
   * `order_id`: The order ID being claimed, a string.
   * `signature`: The signature of the message constructed from the address and order ID, in hexadecimal.

An example of a valid memo is:
```json
{
  "claim": {
    "address": "a8d523755de41b9c14de709ca59d52bc1772658258962ef5bbefa8c59082e54733",
    "order_id": "oid554432",
    "signature": "2421e3c98522d7c5518f55ddb39f759ee9051dde8060679d48f257994372fb214e9024917a5befacb132fc9979527ff92daa2c5d42062b8a507dc4e3b6954c05"
  },
  "other_stuff(optional)": {
    "foo": "bar"
  }
}
```

[MemoSignature]: tari_payment_engine/src/helpers/memo_signature.rs 

## Double step - Pay and Claim

In this flow, the user pays for the order, and claims the order in two separate steps. The steps can be carried out in 
any order. This is useful when 
* the user does not know the order number at the time of payment, 
* when users want to use one-sided payments to the TPS wallet, 
* when the user's wallet does not support generating an order claim.
* when the user dows not know where to send funds (in this case, make the claim first, and use the `send_to` field 
  to complete the payment).

This approach can also be used by wallets to integrate TPS if the user flow is better served by a 2-step process.

## Step 1

Make a payment to the TPS wallet. This payment can be made at any time, and will be credited to the address of the 
sender's wallet.

## Step 2
               
Make a `POST` request to the `/order/claim` endpoint of the Tari Payment Server with the order ID and a claim message.

Specifically, the format of the request body must be a claim object as described [above](#creating-the-signature). 
For example,
    
```json
{
"address": "a8d523755de41b9c14de709ca59d52bc1772658258962ef5bbefa8c59082e54733",
"order_id": "oid554432",
"signature": "2421e3c98522d7c5518f55ddb39f759ee9051dde8060679d48f257994372fb214e9024917a5befacb132fc9979527ff92daa2c5d42062b8a507dc4e3b6954c05"
}
```

If successful, the response will be a `200 OK` with a JSON object containing the order ID and the status of the claim.

```json
{
  "order_id": "oid554432",
  "total_price": 50000000,
  "expires_at": "2024-06-29T14:00:00Z",
  "status": "Claimed",
  "send_to": "addressofhotwallet",
}
```

If unsuccessful, the response will be a `40x` (if a user error) or `50x` (if a backend error) code with a reason for 
the failure.
