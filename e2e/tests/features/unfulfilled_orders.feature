Feature: The /api/unfulfilled_orders endpoint
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

  Scenario: Standard user can access their own unfulfilled orders
    When Customer #1 ["alice"] places order "alice001" for 250 XTR, with memo
    """
    {
      "address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "order_id":"alice001",
      "signature":"a03f9c56f789a19167e964bc9c8cc060842a7664033afd5b9bc6cb2c57f38608d38b890012e4b9d54320054abab75b42635ccffbb98bf6b59e88d6e37185640b"
    }
    """
    When Customer #1 ["alice"] places order "alice002" for 150 XTR, with memo
    """
    {
      "address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "order_id":"alice002",
      "signature":"c2a797a6e690a98f7055c6e27e860d38607027cd9e05a18a5fc3f659222b5f1719ec1c66f82855f824e9009bb2f390196acaa9c8fe5b5620fe34a0484c98f203"
    }
    """
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/unfulfilled_orders" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    [
      { "order_id": "alice001", "total_price": 250000000, "status": "New" },
      { "order_id": "alice002", "total_price": 150000000, "status": "New" }
    ]
    """
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {"payment": {
      "sender":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "amount":250000000,
      "txid":"payment001"
    },
    "auth": {
      "address":"c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495","nonce":1,
      "signature":"22cab0a461b2dcd0fd4f7ac688775b51e365bdde4f8b0f3d977a877e28151c43f3ee321ebcd8ec51b2fd41c022862ae507f4f93d37e77dadbfc3122718a4a10a"}
    }
    """
    Then I receive a 200 Ok response with the message '"success":true'
    # Transaction is not confirmed yet
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
    When Alice GETs to "/api/unfulfilled_orders" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    [ { "order_id": "alice002", "total_price": 150000000, "status": "New" } ]
    """

