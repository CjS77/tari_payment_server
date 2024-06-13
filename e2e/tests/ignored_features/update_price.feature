Feature: Admins can update the price field of an order
  Background:
    Given a database with some accounts
    Given some role assignments

  Scenario: Unauthenticated users cannot access the `/order_price` endpoint
    When User PATCHs to "/api/order_price" with body
        """
        {
          "order_id": "1",
          "new_price": 100
        }
        """
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user cannot access the `/order_price` endpoint
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice PATCHs to "/api/order_price" with body
        """
        {
          "order_id": "1",
          "new_price": 100,
          "reason": "I want to update the price"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Admin user cannot access the `/order_price` endpoint if he does not ask for 'write' role
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin PATCHs to "/api/order_price" with body
        """
        {
          "order_id": "1",
          "new_price": 100
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: An admin can update the price on behalf of a user.
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin POSTs to "/api/credit" with body
    """
    {
      "customer_id": "alice",
      "amount": 45000000,
      "reason": "Not enough to fill an order"
    }
    """
    Then I receive a 200 OK response
    Then Alice has a current balance of 45 Tari
    # Reduce the price, but still not enough for the order to be filled
    When Admin PATCHs to "/api/order_price" with body
    """
    {
      "order_id": "1",
      "new_price": 50000000
    }
    """
    Then I receive a 200 OK response
    Then I receive a partial JSON response:
    """
    {"order_id": "1", "total_price": 50000000 }
    """
    Then the OnOrderModified trigger fires with
    """
    {
      "field_changed": "total_price",
      "orders": {
        "old_order": {"order_id": "1", "total_price": 100000000},
        "new_order": {"order_id": "1", "total_price": 50000000}
      }
    }
    """
    Then Alice has a current balance of 45 Tari
    Then order "1" is in state New
    # Reduce the price again, but this time, the order will be filled
    When Admin PATCHs to "/api/order_price" with body
    """
    {
      "order_id": "1",
      "new_price": 25000000
    }
    """
    Then I receive a 200 OK response
    Then I receive a partial JSON response:
    """
    {"order_id": "1", "total_price": 25000000 }
    """
    Then the OnOrderModified trigger fires with
    """
    {
      "field_changed": "total_price",
      "orders": {
        "old_order": {"order_id": "1", "total_price": 50000000},
        "new_order": {"order_id": "1", "total_price": 25000000}
      }
    }
    """
    Then Alice has a current balance of 20 Tari
    Then order "1" is in state Paid
    # Reduce the other order's price, and fill the order
    When Admin PATCHs to "/api/order_price" with body
    """
    {
      "order_id": "3",
      "new_price": 0
    }
    """
    Then I receive a 200 OK response
    Then I receive a partial JSON response:
    """
    {"order_id": "3", "total_price": 0 }
    """
    Then the OnOrderModified trigger fires with
    """
    {
      "field_changed": "total_price",
      "orders": {
        "old_order": {"order_id": "3", "total_price": 65000000},
        "new_order": {"order_id": "3", "total_price": 0}
      }
    }
    """
    Then Alice has a current balance of 20 Tari
    Then order "3" is in state Paid
    Then account for alice has current orders worth 0 XTR
    Then account for alice has total orders worth 25 XTR

  Scenario: The price must be positive
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin PATCHs to "/api/order_price" with body
    """
    {
      "order_id": "1",
      "new_price": -100
    }
    """
    Then I receive a 400 BadRequest response with the message 'The requested order change is forbidden.'
    Then order "1" is in state New

