Feature: Full accounts endpoint (/api/history)
  Background:
    Given a database with some accounts
    Given some payments are received
    Given some role assignments

  Scenario: Unauthenticated user cannot access the `history` endpoint
    When User GETs to "/api/history" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user can access their own account history
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/history" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
    "account":{
      "id":1,
      "total_received":115000000,
      "current_pending":115000000,
      "current_balance":0,
      "total_orders":165000000,
      "current_orders":165000000
    },
    "addresses":[ {"address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d" } ],
    "customer_ids":[{"customer_id":"alice"}],
    "orders":[
      {"id":1,"order_id":"1","total_price":100000000,"status":"New"},
      {"id":3,"order_id":"3","total_price":65000000,"status":"New"}],
    "payments":[
      {"txid":"alicepayment001","amount":15000000,"status":"received"},
      {"txid":"alicepayment002","amount":100000000,"status":"received"}
    ]
    }
    """

  Scenario: Standard user cannot access another user's account history with an address
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/history/address/680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Standard user cannot access another user's account history with an account id
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/history/id/2" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: User with ReadAll role can access any account history with an address
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin GETs to "/api/history/address/680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "account":{
        "id":2,
        "total_received":550000000,
        "current_pending":550000000,
        "current_balance":0,
        "total_orders":550000000,
        "current_orders":550000000
      },
      "addresses":[ {"address":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b"}],
      "customer_ids":[{"customer_id":"bob"}],
      "orders":[
        {"id":2,"order_id":"2","customer_id":"bob","total_price":200000000,"status":"New"},
        {"id":4,"order_id":"4","customer_id":"bob","total_price":350000000,"status":"New"}
      ],
      "payments":[
        {"txid":"bobpayment001","amount":50000000,"status":"received"},
        {"txid":"bobpayment002","amount":500000000,"status":"received"}
      ]
    }
    """

  Scenario: User with ReadAll role can access any account history with an address
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin GETs to "/api/history/id/2" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "account":{
        "id":2,
        "total_received":550000000,
        "current_pending":550000000,
        "current_balance":0,
        "total_orders":550000000,
        "current_orders":550000000
      },
      "addresses":[ {"address":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b"}],
      "customer_ids":[{"customer_id":"bob"}],
      "orders":[
        {"id":2,"order_id":"2","customer_id":"bob","total_price":200000000,"status":"New"},
        {"id":4,"order_id":"4","customer_id":"bob","total_price":350000000,"status":"New"}
      ],
      "payments":[
        {"txid":"bobpayment001","amount":50000000,"status":"received"},
        {"txid":"bobpayment002","amount":500000000,"status":"received"}
      ]
    }
    """

  Scenario: SuperAdmin role can access another account
    Given a super-admin user (Super)
    When Super authenticates with nonce = 1
    When Super GETs to "/api/history/address/680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "account":{
        "id":2,
        "total_received":550000000,
        "current_pending":550000000,
        "current_balance":0,
        "total_orders":550000000,
        "current_orders":550000000
      },
      "addresses":[ {"address":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b"}],
      "customer_ids":[{"customer_id":"bob"}],
      "orders":[
        {"id":2,"order_id":"2","customer_id":"bob","total_price":200000000,"status":"New"},
        {"id":4,"order_id":"4","customer_id":"bob","total_price":350000000,"status":"New"}
      ],
      "payments":[
        {"txid":"bobpayment001","amount":50000000,"status":"received"},
        {"txid":"bobpayment002","amount":500000000,"status":"received"}
      ]
    }
    """
