@no_strict
Feature: Order Fulfillment with strict mode off
  Background:
    Given a server configuration
      | use_x_forwarded_for          | true  |
      | order_id_field               | name  |
      | disable_memo_signature_check | true  |
      | strict_mode                  | false |
    Given a blank slate
    Given an authorized wallet with secret df158b8389c68aac01a91276b742d2527f951d3c7289e4ccdecfa0672947270e
    """
      {
        "address": "14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
        "ip_address": "192.168.1.100"
      }
    """

  Scenario: Alice: Order -> Pay -> Confirm. The order is fulfilled.
    When Customer #1 ["alice"] places order with name "#1000" for 2400 XTR
    Then customer id 1 has current orders worth 2400 XTR
    And account for customer 1 has a current balance of 0 XTR
    And order "id-#1000" is in state Unclaimed
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {
      "payment": {
         "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
         "amount":2500000000,
         "txid":"payment001",
         "memo": "order 1000"
      },
      "auth": {"address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":1,"signature":"ac509cfdb1915e572faf9fb5c29ee5ab364f177f352f36a8f8b50ab48f34374961103694fea123f30a1e0ec66321186b6c3ead8e5cedf88c64e3256c1610fb03"}
    }
    """
    Then I receive a 200 Ok response with the message '"success":true'
    # Transaction is not confirmed yet
    Then order "id-#1000" is in state New
    And account for customer 1 has a current balance of 0 XTR
    And User Alice has a pending balance of 2500 XTR
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
    Then order "id-#1000" is in state Paid
    And account for customer 1 has a current balance of 100 XTR
    And the OrderPaid trigger fires with
    """
    {
      "order": {
      "order_id": "id-#1000",
      "alt_id": "#1000",
      "customer_id": "1",
      "total_price": 2400000000,
      "currency": "XTR",
      "id": 1,
      "status": "Paid"
    }
   }
   """

  Scenario: Alice: Pay -> Confirm -> Order. The order is fulfilled
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {"payment": {
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "amount":2500000000,
      "txid":"payment001",
      "order_id": "#1001"
    },
    "auth": {"address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":1,"signature":"50d3560c49a9a53ab6dcb094ef647770b457aff5876ad2792df669d7bac40a11089523951e32e5e0134e50917cfa4b49dab1191e73bfe75eb5814d8ffe063009"}
    }
    """
    Then I receive a 200 Ok response with the message '"success":true'
    And account for customer 1 has a current balance of 0 XTR
    And User Alice has a pending balance of 2500 XTR
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
    Then address 14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt has a current balance of 2500 XTR
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
    When Customer #1 ["alice"] places order with name "#1001" for 2400 XTR
    Then customer id 1 has paid orders worth 2400 XTR
    And account for customer 1 has a current balance of 100 XTR
    And order "id-#1001" is in state Paid
    And the OrderPaid trigger fires with
    """
    {
      "order": {
        "order_id": "id-#1001",
        "alt_id": "#1001",
        "customer_id": "1",
        "total_price": 2400000000,
        "currency": "XTR",
        "id": 1,
        "status": "Paid"
      }
    }
    """

  Scenario: Multiple concurrent customers and payments
    When Customer #1 ["alice"] places order with name "#1000" for 2400 XTR
    When Customer #2 ["bob"] places order with name "#2000" for 700 XTR
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
    { "auth": {"address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":2,"signature":"58b7db039c121177607ace2c48a5678a7a662e0feeb9044e830c608426a108728737ef15800399b99c66afc3f677ed0048333041221562b0cfad77715708fd0d"},
    "payment": {
      "sender":"14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp",
      "amount":400000000,
      "txid":"ab34cd56ef90",
      "memo": "#2000"
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
    Then account for customer 2 has a current balance of 400 XTR
    And order "id-#2000" is in state New
    When Customer #3 ["anon"] places order "id-#3000" for 84 XTR, with memo
    """
    {
      "address":"148pD3w44RtpDZ63RxzJoaCJYSUUEpfpFLyacw1qzAgLmo7",
      "order_id":"id-#3000",
      "signature":"8aaeed3e937ccf02b2de789e29675bc11337d783eefd6c8eb3146edf0d92fc11c4871243fcd452a0856b4bf2260e8f10c2ded5b0a5b6cd16306d2ce9d6e8a30f"
    }
    """
    Then account for customer 3 has a current balance of 1 XTR
    And order "id-#3000" is in state Paid
    When Customer #1 ["alice"] places order with name "#1002" for 3600 XTR
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {
    "auth": {"address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":5,"signature":"b81ad88852625658424b03c3268c9c5432748991eeb2b468931adcca321bdc1a5e6b2e63a2afe3b87a1e0453900664fd3945c4ef79e2edc57c8de4e58a1b0807"},
    "payment": {
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "amount":6042000000,
      "txid":"alicebigpayment",
      "memo": "Order number 1002"
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
    Then account for customer 1 has a current balance of 42 XTR
    And order "id-#1002" is in state Paid
    And order "id-#1000" is in state Paid

  Scenario: Order -> Pay with memo-> Confirm. The order is fulfilled.
    When Customer #1 ["alice"] places order with name "#1005" for 2100 XTR
    Then customer id 1 has current orders worth 2100 XTR
    And account for customer 1 has a current balance of 0 XTR
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {"payment": {
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "amount":2500000000,
      "txid":"payment001",
      "memo": "order #1005"
    },
    "auth": {"address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":1,"signature":"8affc83552884fc42d56600cde08bde1df4fa2759b430d8c9a02af666c34342f6856014c01c2618b1cef7a75cfacf3013b46600e2b63d227e5be2fe704441307"}
    }
    """
    Then I receive a 200 Ok response with the message '"success":true'
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
    Then I receive a 200 Ok response with the message '"success":true'
    And order "id-#1005" is in state Paid
    And customer id 1 has current orders worth 0 XTR
    And account for customer 1 has a current balance of 400 XTR

  Scenario: Order -> Pay -> Confirm -> Claim. The order is fulfilled.
    When Customer #1 ["alice"] places order with name "#1006" for 2100 XTR
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
    Then I receive a 200 Ok response with the message '"success":true'
    And order "id-#1006" is in state Unclaimed

    When User POSTs to "/order/claim" with body
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "order_id":"#1006",
      "signature":"02d4bc50a2b12de784652b3d90b82e83f167e44eae78e85d4408ac3731bcab1dd44b7edce6095e16647720977d3bb0326fec38c901a324700645c32b930dcd0b"
    }
    """
    Then order "id-#1006" is in state Paid
    And account for customer 1 has a current balance of 400 XTR

  Scenario: Pay -> Order -> Claim -> Confirm. The order is fulfilled.
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {"payment": {
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "amount":2500000000,
      "txid":"payment001"
    },
    "auth": {"address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":1,"signature":"ba354b7825785f94b38217a0ec5ecb243c3745b2067cd3519c251bbc0645355995b8c7249b2a003ba8c3ef9d4bfef4b1dc116ea43c56a978873fa7ef39ac5e09"}
    }
    """
    Then I receive a 200 Ok response with the message '"success":true'
    When Customer #1 ["alice"] places order with name "#1006" for 2100 XTR
    When User POSTs to "/order/claim" with body
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "order_id":"#1006",
      "signature":"02d4bc50a2b12de784652b3d90b82e83f167e44eae78e85d4408ac3731bcab1dd44b7edce6095e16647720977d3bb0326fec38c901a324700645c32b930dcd0b"
    }
    """
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
    Then I receive a 200 Ok response with the message '"success":true'
    Then order "id-#1006" is in state Paid
    And account for customer 1 has a current balance of 400 XTR
    And account for customer 2 has a current balance of 0 XTR

  Scenario: Alice: Order -> Pay with payment_id -> Confirm. The order is fulfilled.
    When Customer #1 ["alice"] places order with name "#1007" for 2100 XTR
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {
      "payment": {
        "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
        "amount":2500000000,
        "txid":"payment001",
        "memo": "order#: 1007"
      },
      "auth": {"address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":1,"signature":"3cf7d80eed2a7ff3895d599fe7dc12670a1d1b928d3e6f364f73b435e32791056984f41cd1ffccf3aadf113f72848b152e77ecbeb1cb708eb50cd4f951a8e309"}
    }
    """
    Then order "id-#1007" is in state New
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
    Then order "id-#1007" is in state Paid
    And account for customer 1 has a current balance of 400 XTR

  Scenario: Alice: Pay with payment_id -> Confirm -> Order. The order is fulfilled.
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {
      "payment": {
        "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
        "amount":2500000000,
        "txid":"payment001",
        "order_id": "#1008"
      },
      "auth": {"address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":1,"signature":"4ece42f239f2f9340d2ccb38298103df4ecc6cf8e753b1c29d25daab57950866b0ff9260e38ff08f13f05dce332632eb554bac31265f2baebf50a82427c32d09"}
    }
    """
    Then I receive a 200 Ok response with the message '"success":true'
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
    When Customer #1 ["alice"] places order with name "#1008" for 2100 XTR
    Then order "id-#1008" is in state Paid
    And account for customer 1 has a current balance of 400 XTR
