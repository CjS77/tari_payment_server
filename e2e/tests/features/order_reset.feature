Feature: Admins can reset an order to the New state
  Background:
    Given a database with some accounts
    Given some role assignments

  Scenario: Unauthenticated users cannot access the `/reset_order` endpoint
    When User PATCHs to "/api/reset_order/1" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user cannot access the `/reset_order` endpoint
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice PATCHs to "/api/reset_order/1" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Admin user cannot access the `/reset_order` endpoint if he does not ask for 'write' role
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin PATCHs to "/api/reset_order/1" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: An admin can reset a cancelled order on behalf of a user
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin POSTs to "/api/cancel" with body
        """
        {
          "order_id": "1",
          "reason": "Out of stock"
        }
        """
    Then I receive a 200 OK response
    Then order "1" is in state Cancelled
    Then account for alice has total orders worth 65 XTR
    Then account for alice has current orders worth 65 XTR
    When Admin PATCHs to "/api/reset_order/1" with body
    Then I receive a 200 OK response
    Then order "1" is in state New
    Then account for alice has total orders worth 165 XTR
    Then account for alice has current orders worth 165 XTR
    Then Alice has a current balance of 0 Tari

  Scenario: Resetting a new order has no effect
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin PATCHs to "/api/reset_order/1" with body
    Then I receive a 400 BadRequest response
    Then I receive a partial JSON response:
    """
    {"error": "Cannot complete this request. The requested order change is forbidden."}
    """
    Then order "1" is in state New
    Then account for alice has total orders worth 165 XTR
    Then account for alice has current orders worth 165 XTR

Scenario: If the user has credit after a reset, the order will be filled
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin POSTs to "/api/cancel" with body
        """
        {
          "order_id": "3",
          "reason": "Out of stock"
        }
        """
    Then I receive a 200 OK response
    Then order "3" is in state Cancelled
    Then account for alice has total orders worth 100 XTR
    Then account for alice has current orders worth 100 XTR
    # Give Alice 70 Tari
    When Admin POSTs to "/api/credit" with body
        """
        {
          "customer_id": "alice",
          "amount": 70000000,
          "reason": "Checking that resetting an order gets filled"
        }
        """
    Then I receive a 200 OK response
    Then account for alice has total orders worth 100 XTR
    Then account for alice has current orders worth 100 XTR
    Then Alice has a current balance of 70 Tari
    # Ok, now reset the order
    When Admin PATCHs to "/api/reset_order/3" with body
    Then I receive a 200 OK response
    And order "3" is in state Paid
    And account for alice has total orders worth 165 XTR
    And account for alice has current orders worth 100 XTR
    And Alice has a current balance of 5 Tari
    And the NewOrder trigger fires with
    """
    {"order": {
      "order_id": "3",
      "customer_id": "alice",
      "total_price": 65000000,
      "status": "New"
    }}
    """
    And the OrderModified trigger fires with
    """
    { "field_changed": "status",
      "orders": {
        "old_order": {"order_id": "3", "customer_id": "alice", "total_price": 65000000, "status": "Cancelled"},
        "new_order": {"order_id": "3", "customer_id": "alice", "total_price": 65000000, "status": "New"}
      }
    }
    """
    And the OrderPaid trigger fires with
    """
    { "order": {
        "order_id": "3",
        "customer_id": "alice",
        "total_price": 65000000
      }
    }
    """

