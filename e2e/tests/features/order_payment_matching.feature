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

