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
    Then account for alice has current orders worth 165 XTR
    Then account for bob has current orders worth 550 XTR
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
    Then I receive a partial JSON response:
    """
    {
      "orders": {
        "new_order": {"order_id": "1", "customer_id": "bob", "status": "New" },
        "old_order": {"order_id": "1", "customer_id": "alice", "status": "New" }
      },
      "old_account_id": 1,
      "new_account_id": 2,
      "is_filled": false
    }
    """
    Then the OnOrderModified trigger fires with
    """
    {
      "field_changed": "customer_id",
      "orders": {
        "old_order": {"order_id": "1", "customer_id": "alice"},
        "new_order": {"order_id": "1", "customer_id": "bob"}
      }
    }
    """
    Then account for alice has current orders worth 65 XTR
    Then account for bob has current orders worth 650 XTR

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
    Then I receive a partial JSON response:
    """
    {
      "orders": {
        "new_order": {"order_id": "1", "customer_id": "dave", "status": "New" },
        "old_order": {"order_id": "1", "customer_id": "alice", "status": "New" }
      },
      "old_account_id": 1,
      "new_account_id": 4,
      "is_filled": false
    }
    """
    Then the OnOrderModified trigger fires with
    """
    {
      "field_changed": "customer_id",
      "orders": {
        "old_order": {"order_id": "1", "customer_id": "alice"},
        "new_order": {"order_id": "1", "customer_id": "dave"}
      }
    }
    """
    Then account for alice has current orders worth 65 XTR
    Then account for dave has current orders worth 100 XTR


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
    Then account for customer bob has a current balance of 105 Tari
    When Admin PATCHs to "/api/reassign_order" with body
    """
    {
          "order_id": "1",
          "new_customer_id": "bob",
          "reason": "Fat fginr errors amirite?"
    }
    """
    Then I receive a 200 OK response
    Then I receive a partial JSON response:
    """
    {
      "orders": {
        "new_order": {"order_id": "1", "customer_id": "bob", "status": "Paid" },
        "old_order": {"order_id": "1", "customer_id": "alice", "status": "New" }
      },
      "old_account_id": 1,
      "new_account_id": 2,
      "is_filled": true
    }
    """
    Then the OnOrderModified trigger fires with
    """
    {
      "field_changed": "customer_id",
      "orders": {
        "old_order": {"order_id": "1", "customer_id": "alice"},
        "new_order": {"order_id": "1", "customer_id": "bob"}
      }
    }
    """
    Then account for alice has current orders worth 65 XTR
    Then account for bob has total orders worth 650 XTR
    Then account for bob has current orders worth 550 XTR
    Then Bob has a current balance of 5 Tari

