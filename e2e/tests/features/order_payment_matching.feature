Feature: Order Fulfillment
  Background:
    Given a server configuration
      | use_x_forwarded_for | true |
    Given a blank slate
    Given an authorized wallet with secret df158b8389c68aac01a91276b742d2527f951d3c7289e4ccdecfa0672947270e
    """
      {
        "address": "c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495",
        "ip_address": "192.168.1.100"
      }
    """

  Scenario: Alice places an order and pays for it. The order is fulfilled.
    When Customer #1 ["alice"] places order "alice001" for 2400 XTR, with memo
    """
    {
      "address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "order_id":"alice001",
      "signature":"56e39d539f1865742b41993bdc771a2d0c16b35c83c57ca6173f8c1ced34140aeaf32bfdc0629e73f971344e7e45584cbbb778dc98564d0ec5c419e6f9ff5d06"
    }
    """
    Then Customer #1 has current orders worth 2400 XTR
    And Alice has a current balance of 0 Tari
    And order "alice001" is in state New
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {"payment": {
      "sender":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "amount":2500000000,
      "txid":"payment001"
    },
    "auth": {
      "address":"c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495",
      "nonce":1,
      "signature":"06d0cd5b00172990300481ea509de8c2e184595ed32b587b701aff7134279023b7dfde81542b6b383ff9594d6f4f0dea30347d110bbb496f5041738865f5e80c"
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
        "address":"c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495",
        "nonce":2,
        "signature":"c2f727328c7387282b6e65c8fd0ffcd7a03355cb9046e3602991be9991a7860496ec9ecfa85c0463d37b6a3a2261e50964dfcd5127d41fa907c14448b2a4ce0b"
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
      "customer_id": "alice",
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
      "sender":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "amount":2500000000,
      "txid":"payment001"
    },
    "auth": {
      "address":"c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495",
      "nonce":1,
      "signature":"06d0cd5b00172990300481ea509de8c2e184595ed32b587b701aff7134279023b7dfde81542b6b383ff9594d6f4f0dea30347d110bbb496f5041738865f5e80c"
    }}
    """
    Then I receive a 200 Ok response with the message '"success":true'
    And Alice has a current balance of 0 Tari
    And Alice has a pending balance of 2500 Tari
    And the PaymentReceived trigger fires with
    """
      {
        "payment": {
          "sender": "b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
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
        "address":"c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495",
        "nonce":2,
        "signature":"c2f727328c7387282b6e65c8fd0ffcd7a03355cb9046e3602991be9991a7860496ec9ecfa85c0463d37b6a3a2261e50964dfcd5127d41fa907c14448b2a4ce0b"
      }
    }
    """
    Then Alice has a current balance of 2500 Tari
    And Alice has a pending balance of 0 Tari
    And the PaymentConfirmed trigger fires with
    """
      {
        "payment": {
          "sender": "b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
          "txid": "payment001",
          "amount": 2500000000,
          "status": "confirmed"
        }
      }
    """
    When Customer #1 ["alice"] places order "alice001" for 2400 XTR, with memo
    """
    {
      "address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "order_id":"alice001",
      "signature":"56e39d539f1865742b41993bdc771a2d0c16b35c83c57ca6173f8c1ced34140aeaf32bfdc0629e73f971344e7e45584cbbb778dc98564d0ec5c419e6f9ff5d06"
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
        "customer_id": "alice",
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
      "address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "order_id":"alice001",
      "signature":"56e39d539f1865742b41993bdc771a2d0c16b35c83c57ca6173f8c1ced34140aeaf32bfdc0629e73f971344e7e45584cbbb778dc98564d0ec5c419e6f9ff5d06"
    }
    """
    When Customer #2 ["bob"] places order "bob700" for 700 XTR, with memo
    """
    { "address":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
      "order_id":"bob700",
      "signature":"18cf83225c67e9030d7126443af97659d7ac8ca9b1cd95358c7fb2b33cd0d767bfb7c398975408771f0f6ad0832e987b6ba6b913b1932682ccb1b2bc068e1a06"
    }
    """
    # Get a payment from an unknown user
    # Secret key: 0148217c5dd247c6ccb511c3548cc8b6b5075cbae85f4c3743073fc51a6d8301
    # Address: 1a4dd47666cb4010401e50ecf9dcd553bdc4c1b3c96ce028b14032cb7e9c154984
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
     {
       "auth": {
         "address":"c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495",
         "nonce":1,
         "signature":"86988f995e8454cb2892f71e781857025d82af13f88f148d0559741fa43acd7036cee2990190a131f45e9e9448714951edcfe6a55cbd037fdf0f06a301b9900f"
       },
       "payment": {
         "sender":"1a4dd47666cb4010401e50ecf9dcd553bdc4c1b3c96ce028b14032cb7e9c154984",
         "amount":85000000,
         "txid":"a7o844001"
       }
     }
    """
    # Bob pays, but not enough to cover the order
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    { "auth": {
      "address":"c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495","nonce":2,
      "signature":"9257a3df799c2fa416623407dc226e2518eb3847aa9d5a984ee9861db29b2d683e8e900aea49847605dd9e410e9b38c0face612a3e5da6e70bfa0d1dc8ad0e06"
    },
    "payment": {
      "sender":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
      "amount":400000000,
      "txid":"ab34cd56ef90"
      }
    }
    """
    # Confirm the 2 payments
    When a confirmation arrives from x-forwarded-for 192.168.1.100
    """
    { "auth": {
      "address":"c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495","nonce":3,
      "signature":"6c24e6d896e4146d0d03518d082dfea48a953ef0ab277ab1a0ae083a88d5a5430545bb5428aa5b76d91c744c2c3db35e0db6d34b39bde125045985f0bfb4fe00"
      }, "confirmation": {"txid":"ab34cd56ef90"}
    }
    """
    When a confirmation arrives from x-forwarded-for 192.168.1.100
    """
    { "auth": {
      "address":"c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495","nonce":4,
      "signature":"36f0ac75b2dfbc7f99db4b5496d690c97c7940d924b58522b4d0f0025151552c1f691b4f4532835c5cf15ee2c6762d24288ae8f1cb2f22fb4160e628b54d1204"
      }, "confirmation": {"txid":"a7o844001"}
    }
    """
    Then Bob has a current balance of 400 Tari
    And order "bob700" is in state New
    When Customer #3 ["anon"] places order "anon0001" for 84 XTR, with memo
    """
    {
       "address":"1a4dd47666cb4010401e50ecf9dcd553bdc4c1b3c96ce028b14032cb7e9c154984",
       "order_id":"anon0001",
       "signature":"86267d7373487427e0ea7af57ac2201e6b7d553faef34d701b3a2277fde54135fd5b7b073dfc8ba4a3b90589f3719c267c6b27957f18c4bfa31a31cc7f6a9a06"
    }
    """
    Then account for customer anon has a current balance of 1 Tari
    And order "anon0001" is in state Paid
    When Customer #1 ["alice"] places order "alice0002" for 3600 XTR, with memo
    """
    {
      "address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "order_id":"alice0002",
      "signature":"c28b548c3b45086fbfd0690c8201e96b1cd1bf7fedcaeb37eb51f0a418035c1dd924b9aa994ee39ad7a7672c6620153d5aef91b1c783f8135e6e17de8acdf206"
    }
    """
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    { "auth": {
      "address":"c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495","nonce":5,
      "signature":"a4f9e365cb030a6b8d0055f958e63b2155539a13569b12780aa7bbb1107c336bb1399a6e5a6d71eadc843145f61134f0054c1ebad60325e913c9ad3764df860f"
    }, "payment": {
      "sender":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d","amount":6042000000,"txid":"alicebigpayment"
      }
    }
    """
    When a confirmation arrives from x-forwarded-for 192.168.1.100
    """
    { "auth": {
      "address":"c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495","nonce":6,
      "signature":"fc86a27e9d24c6d7294f9bc0c3231fbde136e79d690bf203d2516f195dca172c57ea79f51a32a09b2a113e1d57f50e4c70bb90bfe5b86b56242e433886298b09"
      }, "confirmation": {"txid":"alicebigpayment"}
    }
    """
    Then Alice has a current balance of 42 Tari
    And order "alice0002" is in state Paid
    And order "alice001" is in state Paid


