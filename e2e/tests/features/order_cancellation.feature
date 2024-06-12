Feature: Admins can cancel an order
  Background:
    Given a database with some accounts
    Given some role assignments

  Scenario: Unauthenticated users cannot access the `/cancel` endpoint
    When User POSTs to "/api/cancel" with body
        """
        {
          "order_id": "1",
          "reason": "griefing"
        }
        """
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user cannot access the `/cancel` endpoint
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice POSTs to "/api/cancel" with body
        """
        {
          "order_id": "1",
          "reason": "I want a free option"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Admin user cannot access the `/cancel` endpoint if he does not ask for 'write' role
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin POSTs to "/api/cancel" with body
        """
        {
          "order_id": "1",
          "reason": "He made a huge mistake"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: An admin can cancel an order on behalf of a user
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin POSTs to "/api/cancel" with body
        """
        {
          "order_id": "1",
          "reason": "Out of stock"
        }
        """
    Then I receive a 200 OK response
    Then I receive a partial JSON response:
    """
    {"order_id": "1", "customer_id":"alice", "total_price": 100000000, "status": "Cancelled"}
    """
    Then order "1" is in state Cancelled
    Then account for alice has total orders worth 65 XTR
    Then account for alice has current orders worth 65 XTR
    Then Alice has a current balance of 0 Tari
    Then the OnOrderAnnulled trigger fires with Cancelled status and order
    """
    {"order_id": "1", "customer_id":"alice", "total_price": 100000000, "status": "Cancelled"}
    """

  Scenario: Cancelling an order a second time has no effect
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin POSTs to "/api/cancel" with body
        """
        {
          "order_id": "1",
          "reason": "Reasons"
        }
        """
    Then I receive a 200 OK response
    Then order "1" is in state Cancelled
    When Admin POSTs to "/api/cancel" with body
        """
        {
          "order_id": "1",
          "reason": "More reasons"
        }
        """
    Then I receive a 400 BadRequest response
    Then I receive a partial JSON response:
    """
    {"error": "Cannot complete this request. The requested order change is forbidden."}
    """
    Then order "1" is in state Cancelled
    Then account for alice has total orders worth 65 XTR
    Then account for alice has current orders worth 65 XTR
    Then Alice has a current balance of 0 Tari

Scenario: You cannot cancel a completed order
  When Admin authenticates with nonce = 1 and roles = "write"
  When Admin POSTs to "/api/fulfill" with body
        """
        {
          "order_id": "1",
          "reason": "Redeem voucher"
        }
        """
  Then I receive a 200 OK response
  Then I receive a partial JSON response:
    """
    {"order_id": "1", "customer_id":"alice", "total_price": 100000000, "status": "Paid"}
    """
  Then order "1" is in state Paid
  When Admin POSTs to "/api/cancel" with body
        """
        {
          "order_id": "1",
          "reason": "Cancel after shipping?"
        }
        """
  Then I receive a 400 BadRequest response
  Then I receive a partial JSON response:
    """
    {"error": "Cannot complete this request. The requested order change is forbidden."}
    """
  Then order "1" is in state Paid
