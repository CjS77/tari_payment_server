Feature: Orders endpoint
  Background:
    Given a database with some accounts
    Given some role assignments


  Scenario: Unauthenticated user cannot access the `orders` endpoint
    When User GETs to "/api/orders" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user can access their own orders
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
         "memo":"Manually inserted by Keith",
         "total_price":100000000,
         "currency":"XTR",
         "status":"New"},
        {"id":3,"order_id":"3","customer_id":"alice",
        "memo":"Manually inserted by Sam",
        "total_price":65000000,"currency":"XTR",
        "status":"New"}
      ]}
    """

  Scenario: Standard user cannot access anyone else's orders
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/orders/680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: User with ReadAll role can access another order set
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
         "memo":"Manually inserted by Keith"},
        {"id":3,"order_id":"3","customer_id":"alice","total_price":65000000,"status":"New",
        "memo":"Manually inserted by Sam"}
      ]}
    """

  Scenario: SuperAdmin role can access another account
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

#--------------------------------------------------------------
#                      Orders by ID
#--------------------------------------------------------------

  Scenario: Unauthorized user cannot access the `/order/id/{}` endpoint
    When I GETs to "/api/order/id/1" with body
    Then I receive a 401 Forbidden response with the message 'Please first authenticate with this application.'

  Scenario: Standard user can access their own order by id
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/order/id/1" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "id":1,"order_id":"1","customer_id":"alice",
      "memo":"Manually inserted by Keith",
      "total_price":100000000
    }
    """

  Scenario: Standard user cannot access anyone else's order by id
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/order/id/2" with body
    Then I receive a 200 Ok response with the message 'null'

  Scenario: Standard user cannot enumerate the order/id endpoint
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/order/id/some_random_order" with body
    Then I receive a 200 Ok response with the message 'null'

  Scenario: User with ReadAll role can access another order by id
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin GETs to "/api/order/id/2" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "id":2,"order_id":"2","customer_id":"bob",
      "memo":"Manually inserted by Charlie",
      "total_price":200000000
    }
    """
    When Admin GETs to "/api/order/id/1" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
        "id":1,"order_id":"1","customer_id":"alice",
        "memo":"Manually inserted by Keith",
        "total_price":100000000
    }
    """

  Scenario: SuperAdmin can access any order by id
    Given a super-admin user (Super)
    When Super authenticates with nonce = 1
    When Super GETs to "/api/order/id/2" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "id":2,"order_id":"2","customer_id":"bob",
      "total_price":200000000
    }
    """
    When Super GETs to "/api/order/id/1" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
        "id":1,"order_id":"1","customer_id":"alice",
        "total_price":100000000
    }
    """
