@order_fulfillment
Feature: Order Fulfillment
  Background:
    Given a server configuration
      | use_x_forwarded_for | true |
    Given a blank slate
    Given an authorized wallet with secret df158b8389c68aac01a91276b742d2527f951d3c7289e4ccdecfa0672947270e
    """
      {
        "address": "14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
        "ip_address": "192.168.1.100"
      }
    """

  Scenario: Alice places an order and pays for it. The order is fulfilled.
    When Customer #1 ["alice"] places order "alice001" for 2400 XTR, with memo
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "order_id":"alice001",
      "signature":"92e9d026e3a4e785ade1ab81e69204bf30c256966964f8f048ec9f06018f1c00ab7ff501a5e0bd7135f38d3e631bc57f851e6f0788f9edc0f908a42d16047701"
    }
    """
    Then Customer #1 has current orders worth 2400 XTR
    And Alice has a current balance of 0 Tari
    And order "alice001" is in state New
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {"payment": {
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "amount":2500000000,
      "txid":"payment001"
    },
    "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
      "nonce":1,
      "signature":"5ed91d58e589bea2f2291155b78005b48a789c3018fdf528f6f4052b2428e532953aa065e08ef91575a610b2ebcf88e63fbf2e21bfd0e8b90b778c16ffc2740e"
    }}
    """
    Then I receive a 200 Ok response with the message '"success":true'
    # Transaction is not confirmed yet
    Then order "alice001" is in state New
    And Alice has a current balance of 0 Tari
    And Alice has a pending balance of 2500 Tari
    When a confirmation arrives from x-forwarded-for 192.168.1.100
    """
    { "confirmation": {"txid": "payment001"},
      "auth": {
        "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
        "nonce":2,
        "signature":"16729e3c9b08022a16edfa0bf2e1cfc1d8dadd5e6387b4fd76005ac8a20c0f53087434524748981e9c61f9b7ce122fdf1a45299b132cd4495b19b7d1208cea01"
      }
    }
    """
    Then order "alice001" is in state Paid
    And Alice has a current balance of 100 Tari
    And the OrderPaid trigger fires with
    """
    {
      "order": {
      "order_id": "alice001",
      "customer_id": "1",
      "total_price": 2400000000,
      "currency": "XTR",
      "id": 1,
      "status": "Paid"
    }
   }
   """

  Scenario: Alice can deposit funds before making orders
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {"payment": {
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "amount":2500000000,
      "txid":"payment001"
    },
    "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
      "nonce":1,
      "signature":"6e48cb91e4a94b2dd04305f81932ed61436a635a44536ab513d430464b946f594fd6e29ddd511d5c756cabb27232301cf6ad625f875f08b44b4346f29b151b0e"
    }}
    """
    Then I receive a 200 Ok response with the message '"success":true'
    And Alice has a current balance of 0 Tari
    And Alice has a pending balance of 2500 Tari
    And the PaymentReceived trigger fires with
    """
      {
        "payment": {
          "sender": "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
          "txid": "payment001",
          "amount": 2500000000,
          "status": "received"
        }
      }
    """
    When a confirmation arrives from x-forwarded-for 192.168.1.100
    """
    { "confirmation": {"txid": "payment001"},
      "auth": {
        "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
        "nonce":2,
        "signature":"3a9edfc5d607943e78557bf220c199486b4cb1964dee2e5be5c3173c40e4e1684131ec88b317a61ea0574b3cbb8ffdec6bb5f4937067b445eabdc3e15121a003"
      }
    }
    """
    Then Alice has a current balance of 2500 Tari
    And Alice has a pending balance of 0 Tari
    And the PaymentConfirmed trigger fires with
    """
      {
        "payment": {
          "sender": "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
          "txid": "payment001",
          "amount": 2500000000,
          "status": "confirmed"
        }
      }
    """
    When Customer #1 ["alice"] places order "alice001" for 2400 XTR, with memo
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "order_id":"alice001",
      "signature":"92e9d026e3a4e785ade1ab81e69204bf30c256966964f8f048ec9f06018f1c00ab7ff501a5e0bd7135f38d3e631bc57f851e6f0788f9edc0f908a42d16047701"
    }
    """
    Then Customer #1 has current orders worth 0 XTR
    And Alice has a current balance of 100 Tari
    And order "alice001" is in state Paid
    And the OrderPaid trigger fires with
    """
    {
      "order": {
        "order_id": "alice001",
        "customer_id": "1",
        "total_price": 2400000000,
        "currency": "XTR",
        "id": 1,
        "status": "Paid"
      }
    }
    """

  Scenario: Multiple concurrent customers and payments
    When Customer #1 ["alice"] places order "alice001" for 2400 XTR, with memo
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "order_id":"alice001",
      "signature":"56e39d539f1865742b41993bdc771a2d0c16b35c83c57ca6173f8c1ced34140aeaf32bfdc0629e73f971344e7e45584cbbb778dc98564d0ec5c419e6f9ff5d06"
    }
    """
    When Customer #2 ["bob"] places order "bob700" for 700 XTR, with memo
    """
    { "address":"14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp",
      "order_id":"bob700",
      "signature":"5ea6273f959276b0c1938efb9688aa9d08eec76688a92b4c626c9329dc4bab4afa2c3aac67e55d1cb036ba099cd6cdf0e42f1fdc6407e78e166c22ec4921ac01"
    }
    """
    # Get a payment from an unknown user
    # Secret key: 0148217c5dd247c6ccb511c3548cc8b6b5075cbae85f4c3743073fc51a6d8301
    # Address: 148pD3w44RtpDZ63RxzJoaCJYSUUEpfpFLyacw1qzAgLmo7
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
     {
       "auth": {
         "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
         "nonce":1,
         "signature":"72e1a01b1f8850c55f15898c1b7d0adc6c57764ab67da65081645a0015da4465e7dba7f4308ec5881b85b09f4e2065aab9652898303c652593770ee03e86cb06"
       },
       "payment": {
         "sender":"148pD3w44RtpDZ63RxzJoaCJYSUUEpfpFLyacw1qzAgLmo7",
         "amount":85000000,
         "txid":"a7o844001"
       }
     }
    """
    # Bob pays, but not enough to cover the order
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    { "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":2,
      "signature":"8695eb40b1bcfa9a019f87f018cf0f753ed3f37407f28ca97093f6f99f8c8e2fbfffec3dcfa009d801369448e9ed9070d67064346453f2cd419792ff149ba00b"
    },
    "payment": {
      "sender":"14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp",
      "amount":400000000,
      "txid":"ab34cd56ef90"
      }
    }
    """
    # Confirm the 2 payments
    When a confirmation arrives from x-forwarded-for 192.168.1.100
    """
    { "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":3,
      "signature":"9ede42f17279ad27683c5bfcfca4f0eb28227bc411811629a9669c681985f255728bc81799546289591020ebb854c7564014249ed3f62cb8b8222334e8ff1f00"
      }, "confirmation": {"txid":"ab34cd56ef90"}
    }
    """
    When a confirmation arrives from x-forwarded-for 192.168.1.100
    """
    { "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":4,
      "signature":"5c8eb63b074d27fe2d75d7eb52613b177b4ca39ec2247d95b0ae71446638544e0ed4a952505e21913f74ea0bdf4396d5a2a60ca359b2889ceccf324c38f39d0b"
      }, "confirmation": {"txid":"a7o844001"}
    }
    """
    Then Bob has a current balance of 400 Tari
    And order "bob700" is in state New
    When Customer #3 ["anon"] places order "anon0001" for 84 XTR, with memo
    """
    {
       "address":"148pD3w44RtpDZ63RxzJoaCJYSUUEpfpFLyacw1qzAgLmo7",
       "order_id":"anon0001",
       "signature":"104baa540fa134d77dad4f1573238989be0d923f560b107e7938790d08e7cb73d3fa684865601b75b2cd6b747995327f194a0cfa96f7449cc449ed84f25f4a05"
    }
    """
    Then account for customer 3 has a current balance of 1 Tari
    And order "anon0001" is in state Paid
    When Customer #1 ["alice"] places order "alice0002" for 3600 XTR, with memo
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "order_id":"alice0002",
      "signature":"9c613426a8dc256b378b63e083b3bd6d1b511fefc033975f543587955bec8b3fac51e7f871f99825cfa98b2e93c4dd5dc48af5a06cfb911b80fae287b78e5a0a"
    }
    """
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    { "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":5,
      "signature":"46b8e6eab217a31d806eff207ca45d78b1e5c49d16516764c827d7d257fcb468e53e10fd1a1cfcdeec4baabfceaeb6ee6f97bf12503e8ee3309cdd8ed1742d05"
    }, "payment": {
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt","amount":6042000000,"txid":"alicebigpayment"
      }
    }
    """
    When a confirmation arrives from x-forwarded-for 192.168.1.100
    """
    { "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":6,
      "signature":"84b80c5f052c7b1d30613bc5ac7772b75d319252ed338c50b87cd77281780471384044b7f7ecfed1ec19c7dc1e49553f86121bfcd682d88926b588f9639e360a"
      }, "confirmation": {"txid":"alicebigpayment"}
    }
    """
    Then Alice has a current balance of 42 Tari
    And order "alice0002" is in state Paid
    And order "alice001" is in state Paid


