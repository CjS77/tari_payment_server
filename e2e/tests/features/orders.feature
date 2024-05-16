Feature: Orders endpoint
  Background:
    Given a database with some accounts

  Scenario: Unauthenticated user cannot access the `orders` endpoint
    When User GETs to "/api/orders" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user can access their own orders
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/orders" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "total_orders":165000000,
      "orders":[
        {"id":1,"order_id":"1","customer_id":"alice",
         "memo":"address: [b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d]",
         "total_price":100000000,
         "currency":"XTR",
         "status":"New"},
        {"id":3,"order_id":"3","customer_id":"alice",
        "memo":"address: [b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d]",
        "total_price":65000000,"currency":"XTR",
        "status":"New"}
      ]}
    """

  Scenario: Standard user cannot access anyone else's orders
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/orders/680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: User with ReadAll role can access another order set
    Given some role assignments
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin GETs to "/api/orders/b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "total_orders":165000000,
      "orders":[
        {"id":1,"order_id":"1","customer_id":"alice","total_price":100000000,"status":"New",
         "memo":"address: [b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d]"},
        {"id":3,"order_id":"3","customer_id":"alice","total_price":65000000,"status":"New",
        "memo":"address: [b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d]"}
      ]}
    """

  Scenario: SuperAdmin role can access another account
    Given some role assignments
    Given a super-admin user (Super)
    When Super authenticates with nonce = 1
    When Super GETs to "/api/orders/680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
      "total_orders":550000000,
      "orders":[
        {"order_id":"2","customer_id":"bob","total_price":200000000,"status":"New"},
        {"order_id":"4","customer_id":"bob","total_price":350000000,"status":"New"}
      ]
    }
    """
