@settle
Feature: Settle accounts
  Background:
    Given a database with some accounts

  Scenario: Unuathenticated user cannot access the `settle_address` endpoint
      When User POSTs to "/api/settle/address/14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt" with body
      Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Unuathenticated user cannot access the `settle_customer` endpoint
    When User POSTs to "/api/settle/customer/alice" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user can settle their own address
    Given some role assignments
    # Does not cause order matching
    When a direct payment of 65 XTR is placed in Alice's account
    Then order "3" is in state New
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice POSTs to "/api/settle" with body
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    {
      "orders_paid":[
        {"order_id":"3","customer_id":"alice","total_price":65000000,"status":"Paid"}
      ],
      "settlements":[
        {"id":1,"order_id":"3","payment_address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt","settlement_type":"single","amount":65000000}
      ]
    }
    """
    And the OrderPaid trigger fires with
    """
    {
      "order": {
        "order_id": "3",
        "customer_id": "alice",
        "total_price": 65000000,
        "currency": "XTR",
        "status": "Paid"
      }
    }
    """
    And order "3" is in state Paid


  Scenario: Standard user cannot settle someone else's address
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice POSTs to "/api/settle/address/14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Standard user cannot settle customer id
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice POSTs to "/api/settle/customer/bob" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Admin user can settle someone else's address
    Given some role assignments
    When Admin authenticates with nonce = 1 and roles = "write"
    When a direct payment of 65 XTR is placed in Alice's account
    # Because the direct deposit does not trigger matching, the order is still unpaid...
    Then order "3" is in state New
    When Admin POSTs to "/api/settle/address/14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt" with body
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    {
      "orders_paid":[
        {"order_id":"3","customer_id":"alice","total_price":65000000,"status":"Paid"}
      ],
      "settlements":[
        {"id":1,"order_id":"3","payment_address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt","settlement_type":"single","amount":65000000}
      ]
    }
    """
    And the OrderPaid trigger fires with
    """
    {
      "order": {
        "order_id": "3",
        "customer_id": "alice",
        "total_price": 65000000,
        "currency": "XTR",
        "status": "Paid"
      }
    }
    """
    And order "3" is in state Paid

  Scenario: Admin user can settle someone else's customer id
    Given some role assignments
    When Admin authenticates with nonce = 1 and roles = "write"
    When a direct payment of 65 XTR is placed in Alice's account
    # Because the direct deposit does not trigger matching, the order is still unpaid...
    Then order "3" is in state New
    When Admin POSTs to "/api/settle/customer/alice" with body
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    {
      "orders_paid":[
        {"order_id":"3","customer_id":"alice","total_price":65000000,"status":"Paid"}
      ],
      "settlements":[
        {"id":1,"order_id":"3","payment_address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt","settlement_type":"single","amount":65000000}
      ]
    }
    """
    And the OrderPaid trigger fires with
    """
    {
      "order": {
        "order_id": "3",
        "customer_id": "alice",
        "total_price": 65000000,
        "currency": "XTR",
        "status": "Paid"
      }
    }
    """
    And order "3" is in state Paid
