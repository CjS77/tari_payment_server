Feature: Admins can assign an order to a new customer
  Background:
    Given a database with some accounts
    Given some role assignments

  Scenario: Unauthenticated users cannot access the `/reassign_order` endpoint
    When User PATCHs to "/api/reassign_order" with body
        """
        {
          "order_id": "1",
          "new_customer_id": "bob"
        }
        """
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user cannot access the `/reassign_order` endpoint
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice PATCHs to "/api/reassign_order" with body
        """
        {
          "order_id": "1",
          "new_customer_id": "bob"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Admin user cannot access the `/reassign_order` endpoint if he does not ask for 'write' role
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin PATCHs to "/api/reassign_order" with body
        """
        {
          "order_id": "1",
          "new_customer_id": "bob",
          "reason": "You must ask for the WRITE roles, geddit?"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: An admin can move an order to a new user.
    Then customer id alice has current orders worth 165 XTR
    Then customer id bob has current orders worth 550 XTR
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin PATCHs to "/api/reassign_order" with body
    """
    {
          "order_id": "1",
          "new_customer_id": "bob",
          "reason": "User moved to new wallet"
    }
    """
    Then I receive a 200 OK response
    And I receive a partial JSON response:
    """
    {
      "orders": {
        "new_order": {"order_id": "1", "customer_id": "bob", "status": "New" },
        "old_order": {"order_id": "1", "customer_id": "alice", "status": "New" }
      },
      "settlements": []
    }
    """
    And the OrderModified trigger fires with
    """
    {
      "field_changed": "customer_id",
      "orders": {
        "old_order": {"order_id": "1", "customer_id": "alice"},
        "new_order": {"order_id": "1", "customer_id": "bob"}
      }
    }
    """
    And customer id alice has current orders worth 65 XTR
    And customer id bob has current orders worth 650 XTR

  Scenario: Assigning an order to a non-existent customer creates a new account
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin PATCHs to "/api/reassign_order" with body
    """
    {
          "order_id": "1",
          "new_customer_id": "dave",
          "reason": "User moved to new wallet"
    }
    """
    Then I receive a 200 OK response
    And I receive a partial JSON response:
    """
    {
      "orders": {
        "old_order": {
          "id": 1,
          "order_id": "1",
          "customer_id": "alice",
          "status": "New"
        },
        "new_order": {
          "id": 1,
          "order_id": "1",
          "customer_id": "dave",
          "status": "New"
        }
      },
      "settlements": []
    }
    """
    And the OrderModified trigger fires with
    """
    {
      "field_changed": "customer_id",
      "orders": {
        "old_order": {"order_id": "1", "customer_id": "alice"},
        "new_order": {"order_id": "1", "customer_id": "dave"}
      }
    }
    """
    And customer id alice has current orders worth 65 XTR
    And customer id dave has current orders worth 100 XTR


  Scenario: Assigning an order to the same customer returns an error
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin PATCHs to "/api/reassign_order" with body
    """
    {
          "order_id": "1",
          "new_customer_id": "alice",
          "reason": "Dancing with myself"
    }
    """
    Then I receive a 400 BadRequest response with the message 'The requested order change would result in a no-op.'

  @fails
  Scenario: Assigning an order to a customer with credit fills the order
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin POSTs to "/api/credit" with body
    """
    {
      "customer_id": "bob",
      "amount": 105000000,
      "reason": "Covering order 1"
    }
    """
    Then account for customer bob has a current balance of 105 XTR
    When Admin PATCHs to "/api/reassign_order" with body
    """
    {
          "order_id": "1",
          "new_customer_id": "bob",
          "reason": "Fat fginr errors amirite?"
    }
    """
    Then I receive a 200 OK response
    And I receive a partial JSON response:
    """
    {
      "orders": {
        "new_order": {"order_id": "1", "customer_id": "bob", "status": "Paid" },
        "old_order": {"order_id": "1", "customer_id": "alice", "status": "New" }
      },
      "settlements": [
        {"order_id": "1", "payment_address": "13111eLuVvxBvAAf3tKJFWKNhJrv5e1dV4on8e3AW1Qq4e", "amount": 100000000 }
      ]
    }
    """
    Then the OrderModified trigger fires with
    """
    {
      "field_changed": "customer_id",
      "orders": {
        "old_order": {"order_id": "1", "customer_id": "alice"},
        "new_order": {"order_id": "1", "customer_id": "bob"}
      }
    }
    """
    And the OrderPaid trigger fires with
    """
    { "order": {
        "order_id": "1",
        "customer_id": "bob",
        "total_price": 100000000
      }
    }
    """
    And customer id alice has current orders worth 65 XTR
    And customer id bob has current orders worth 550 XTR
    And account for customer bob has a current balance of 5 XTR

