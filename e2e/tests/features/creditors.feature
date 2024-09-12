@creditors
Feature: Creditors
  Background:
    Given a database with some accounts
    Given some role assignments
    Given some payments are received

    Scenario: Admins with the ReadAll role can see all creditors
      When Admin authenticates with nonce = 1 and roles = "read_all"
      When Admin GETs to "/api/creditors" with body
      Then I receive a 200 Ok response
      Then I receive a partial JSON response:
      """
      [
        {"customer_id":"admin","status":"New","total_orders":25000000},
        {"customer_id":"alice","status":"New","total_orders":165000000},
        {"customer_id":"bob","status":"New","total_orders":550000000}
      ]
      """
      When payment alicepayment001 is confirmed
      When payment bobpayment001 is confirmed
      When Admin GETs to "/api/creditors" with body
      Then I receive a 200 Ok response
      Then I receive a partial JSON response:
      """
      [
        {"customer_id":"admin","status":"New","total_orders":25000000},
        {"customer_id":"alice","status":"New","total_orders":165000000},
        {"customer_id":"bob","status":"New","total_orders":550000000}
      ]
      """

      # This will cover the 100XTR order, but not the 65 XTR order as well
      When payment alicepayment002 is confirmed
      # This covers both Bob's orders, and he has zero balance left, so not a creditor anymore
      When payment bobpayment002 is confirmed
      # No orders for this user, just a positive current balance now
      When payment anonpayment001 is confirmed
      When Admin GETs to "/api/creditors" with body
      Then I receive a 200 Ok response
      Then I receive a partial JSON response:
      """
      [
        {"customer_id":"admin","status":"New","total_orders":25000000},
        {"customer_id":"alice","status":"New","total_orders":65000000}
      ]
      """

  Scenario: Unauthenticated user cannot access the `creditors` endpoint
    When User GETs to "/api/creditors" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user cannot access the `creditors` endpoint
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/creditors" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'


