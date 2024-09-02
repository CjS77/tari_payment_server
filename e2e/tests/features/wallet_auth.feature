@wallet_auth
Feature: Wallet Authorization
  Background:
    Given a server configuration
    | use_x_forwarded_for | true |
    | use_forwarded       | true |
    Given a blank slate
    Given an authorized wallet with secret df158b8389c68aac01a91276b742d2527f951d3c7289e4ccdecfa0672947270e
    """
      {
        "address": "14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
        "ip_address": "192.168.1.100"
      }
    """

  Scenario: Wallet Authorization from wrong IP address fails
    When a payment arrives from x-forwarded-for 1.2.3.4
    """
    {"payment": {
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "amount":2500000000,
      "txid":"payment001"
    },
    "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
      "nonce":1,
      "signature":"06d0cd5b00172990300481ea509de8c2e184595ed32b587b701aff7134279023b7dfde81542b6b383ff9594d6f4f0dea30347d110bbb496f5041738865f5e80c"
    }}
    """
    Then I receive a 401 Unauthorized response with the message ''

  Scenario: Wallet Authorization from correct IP address passes (x-forwarded-for)
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
      "signature":"f059d52b25d3f1387b8a9becca8775c1ff9a62b7293ba3791f78e77ac63a786e3c02534529809d4dd4ae8ccafeae28c6b2e4a105baf2fa9ebad44776978d4b02"
    }}
    """
    Then I receive a 200 Ok response with the message '"success":true'

  Scenario: Wallet Authorization from correct IP address passes (forwarded)
    When a payment arrives from forwarded 192.168.1.100
    """
    {"payment": {
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "amount":2500000000,
      "txid":"payment001"
    },
    "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
      "nonce":1,
      "signature":"f059d52b25d3f1387b8a9becca8775c1ff9a62b7293ba3791f78e77ac63a786e3c02534529809d4dd4ae8ccafeae28c6b2e4a105baf2fa9ebad44776978d4b02"
    }}
    """
    Then I receive a 200 Ok response with the message '"success":true'

  Scenario: Wallet Authorization from correct IP address has incorrect signature
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    { "payment": { "sender": "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt", "amount": 500000000, "txid": "payment001", "order_id": "order001" },
      "auth": { "address": "14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG", "nonce": 1,
      "signature": "bad570a3da2b8d233d4d5e12e54d71b8b0a5be8cf56a878fb078f3757a07417fd64d61ae002276e96893d47d725085552bc15babb2488836af39a408a07c5200" } }
    """
    Then I receive a 401 Ok response with the message ''

  Scenario: Wallet Authorization from correct IP address passes has tampered payment
    When a payment arrives from forwarded 192.168.1.100
    """
    {"payment": {
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "amount":99999999999,
      "txid":"payment001"
    },
    "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
      "nonce":1,
      "signature":"06d0cd5b00172990300481ea509de8c2e184595ed32b587b701aff7134279023b7dfde81542b6b383ff9594d6f4f0dea30347d110bbb496f5041738865f5e80c"
    }}
    """
    Then I receive a 401 Ok response with the message ''



