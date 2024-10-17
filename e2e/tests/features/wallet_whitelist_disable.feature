@wallet_whitelist
Feature: Wallet whitelist disable
  Background:
    Given a server configuration
      | use_x_forwarded_for | true |
      | use_forwarded       | true |
      | disable_wallet_whitelist | true |
    Given a blank slate
    Given an authorized wallet with secret df158b8389c68aac01a91276b742d2527f951d3c7289e4ccdecfa0672947270e
    """
      {
        "address": "14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
        "ip_address": "192.168.1.100"
      }
    """

  Scenario: Wallet Authorization from wrong IP address passes because whitelists are disabled
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
      "signature":"f059d52b25d3f1387b8a9becca8775c1ff9a62b7293ba3791f78e77ac63a786e3c02534529809d4dd4ae8ccafeae28c6b2e4a105baf2fa9ebad44776978d4b02"
    }}
    """
    Then I receive a 200 Ok response with the message '"success":true'
