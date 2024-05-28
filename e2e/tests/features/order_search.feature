Feature: Admins can search orders by various criteria
  Background:
    Given a database with some accounts
    Given some role assignments
    When Admin authenticates with nonce = 1 and roles = "read_all"

  Scenario: Admin can search for orders after a certain date
    When Admin GETs to "/api/search/orders?since=2024-03-11T0:0:0Z" with body
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    [
      { "order_id": "3", "customer_id": "alice","created_at":"2024-03-11T16:00:00Z"},
      { "order_id": "4", "customer_id": "bob",  "created_at":"2024-03-11T17:00:00Z"},
      { "order_id": "5", "customer_id": "admin","created_at":"2024-03-12T18:00:00Z"}
    ]
    """

  Scenario: Admin can search for orders before a certain date
    When Admin GETs to "/api/search/orders?until=2024-03-11T0:0:0Z" with body
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    [
      { "order_id": "1", "customer_id": "alice","created_at":"2024-03-10T15:00:00Z"},
      { "order_id": "2", "customer_id": "bob",  "created_at":"2024-03-10T15:30:00Z"}
    ]
    """

  Scenario: Admin can search for orders between given dates
    When Admin GETs to "/api/search/orders?since=2024-03-10T15:15:0Z&until=2024-03-11T16:15:0Z" with body
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    [ { "order_id": "2"}, { "order_id": "3"} ]
    """

#  Scenario: Admin can search for orders with a given status
#    When todo
#
#  Scenario: Admin can search for orders with two given statuses
#    When todo

  Scenario: Admin can search for orders with a matching memo field
    When Admin GETs to "/api/search/orders?memo=Charlie" with body
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    [ { "order_id": "2"}, { "order_id": "5"} ]
    """

  Scenario: Admin can search for orders with a matching customer id
    When Admin GETs to "/api/search/orders?customer_id=alice" with body
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    [ { "order_id": "1"}, { "order_id": "3"} ]
    """

  Scenario: Admin can search for orders with a matching currency
    When Admin GETs to "/api/search/orders?currency=XMR" with body
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    [ { "order_id": "5"} ]
    """

  Scenario: Admin can search for orders with a matching order id
    When Admin GETs to "/api/search/orders?order_id=2" with body
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    [ { "order_id": "2"} ]
    """

  Scenario: Admin can search for orders using multiple criteria
    When Admin GETs to "/api/search/orders?customer_id=bob&since=2024-03-11T0:0:0Z" with body
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    [ { "order_id": "4"} ]
    """
