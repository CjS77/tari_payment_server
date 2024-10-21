@memo_signature_disable
Feature: Order id using order name
  Background:
    Given a server configuration
      | use_x_forwarded_for          | true |
      | order_id_field               | id   |
      | disable_memo_signature_check | true |
    Given a blank slate
    Given an authorized wallet with secret df158b8389c68aac01a91276b742d2527f951d3c7289e4ccdecfa0672947270e
    """
      {
        "address": "14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
        "ip_address": "192.168.1.100"
      }
    """

  Scenario: Alice: Pay with payment_id -> Confirm -> Order. The order is fulfilled.
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {"payment": {
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "amount":2500000000,
      "txid":"payment001",
      "memo": "order #12345"
    },
    "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
      "nonce":1,
      "signature":"8e3b91d09b4118053af3cd1ed508cf8771cfcad86712e6ff1c38f6cafcb8954c532af250c3a8d5d990f0af66af5372325e5dac08b14f2ec0a643df96060f3300"
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
    When Customer #1 ["alice"] places order "12345" for 2400 XTR, with memo
    Then order "12345" is in state Paid
    And account for customer 1 has a current balance of 100 XTR

