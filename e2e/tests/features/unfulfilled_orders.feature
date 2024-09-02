@unfulfilled_orders
Feature: The /api/unfulfilled_orders endpoint
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
    When Customer #1 ["alice"] places order "alice001" for 250 XTR, with memo
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "order_id":"alice001",
      "signature":"601d2f88738b9aacdf75b62de9db23bb8d1aeb9fef8c7d127a05e9648729e64143adcee0a3d50665eb9c7817a0035ad9ac6248a8547bd6d5a4796b917bd57d09"
    }
    """
    When Customer #2 ["bob"] places order "bob001" for 999 XTR, with memo
    """
    {
      "address":"14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp",
      "order_id":"bob001",
      "signature":"4c5f0fa0ffbf4064ee8a50a7033b83143209568581d28a0983059e46cbc7dd5e023813ea97fea351380c4528a73a2400ba929b65bd2dd538951dcc32ae3d9102"
    }
    """
    When Customer #1 ["alice"] places order "alice002" for 150 XTR, with memo
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "order_id":"alice002",
      "signature":"28828b5ca8a693da33f4c98471295c6671c27f09053c101fefac21f96f280931398aaef0bd76b6df9c6c5a4cebb2b47865f1e4e34dc6bed0f0f4aa8da1827901"
    }
    """

  Scenario: Standard user can access their own (and only their own) unfulfilled orders
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
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "amount":250000000,
      "txid":"payment001"
    },
    "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":1,
      "signature":"22382015f53740112c8f4455e0716bd20c915f10c230a245927913cf3297c03d07596b8c5ca2fbc5024edd311198ed7e066d6adbc6f8cd91ebc0cce1e1eb5006"}
    }
    """
    Then I receive a 200 Ok response with the message '"success":true'
    # Transaction is not confirmed yet
    When a confirmation arrives from x-forwarded-for 192.168.1.100
    """
    { "confirmation": {"txid": "payment001"},
      "auth": {
        "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
        "nonce":2,
        "signature":"fc1f12cb448b4c6184f11db5e28de911b6f53ca8120747d268405cd5c180c2212d61aa51bbfc00d26d0e2b8134eff6262fcdfedbf8309555bba1cb52bc424f04"
      }
    }
    """
    When Alice GETs to "/api/unfulfilled_orders" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    [ { "order_id": "alice002", "total_price": 150000000, "status": "New" } ]
    """

  Scenario: Admin users with ReadAll role can access any unfulfilled orders
    Given some role assignments
    When Admin authenticates with nonce = 1 and roles = "read_all"
    When Admin GETs to "/api/unfulfilled_orders/14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address": "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp",
      "total_orders": 999000000,
      "orders": [ { "order_id": "bob001", "total_price": 999000000, "status": "New" } ]
    }
    """
    When Admin GETs to "/api/unfulfilled_orders/14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address": "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "total_orders": 400000000,
      "orders": [ { "order_id": "alice001", "total_price": 250000000, "status": "New" }, { "order_id": "alice002", "total_price": 150000000, "status": "New" } ]
    }
    """
    # Now one of the orders gets fulfilled
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {"payment": {
      "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "amount":250000000,
      "txid":"payment001"
    },
    "auth": {
      "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":1,
      "signature":"22382015f53740112c8f4455e0716bd20c915f10c230a245927913cf3297c03d07596b8c5ca2fbc5024edd311198ed7e066d6adbc6f8cd91ebc0cce1e1eb5006"}
    }
    """
    Then I receive a 200 Ok response with the message '"success":true'
    # Transaction is not confirmed yet
    When a confirmation arrives from x-forwarded-for 192.168.1.100
    """
    { "confirmation": {"txid": "payment001"},
      "auth": {
        "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
        "nonce":2,
        "signature":"fc1f12cb448b4c6184f11db5e28de911b6f53ca8120747d268405cd5c180c2212d61aa51bbfc00d26d0e2b8134eff6262fcdfedbf8309555bba1cb52bc424f04"
      }
    }
    """
    When Admin GETs to "/api/unfulfilled_orders/14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address": "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "total_orders": 150000000,
      "orders": [ { "order_id": "alice002", "total_price": 150000000, "status": "New" } ]
    }
    """
