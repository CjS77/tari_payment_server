Feature: Admins can mark an order as Paid
  Background:
    Given a database with some accounts
    Given some role assignments

  Scenario: Unauthenticated users cannot access the `/fulfill` endpoint
    When User POSTs to "/api/fulfill" with body
        """
        {
          "order_id": "1",
          "reason": "cause I wanna"
        }
        """
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user cannot access the `/fulfill` endpoint
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice POSTs to "/api/fulfill" with body
        """
        {
          "order_id": "1",
          "reason": "I like free stuff"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Admin user cannot access the `/fulfill` endpoint if he does not ask for 'write' role
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin POSTs to "/api/fulfill" with body
        """
        {
          "order_id": "1",
          "reason": "Refund"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: An admin can mark an existing order as Paid on behalf of a user
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
    Then customer id alice has paid orders worth 100 XTR
    Then customer id alice has current orders worth 65 XTR
    Then account for customer alice has a current balance of 0 XTR

  Scenario: Marking an order Paid a second time has no effect
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin POSTs to "/api/fulfill" with body
        """
        {
          "order_id": "1",
          "reason": "Redeem voucher"
        }
        """
    Then I receive a 200 OK response
    Then order "1" is in state Paid
    When Admin POSTs to "/api/fulfill" with body
        """
        {
          "order_id": "1",
          "reason": "Redeem voucher again"
        }
        """
    Then I receive a 400 BadRequest response
    Then I receive a partial JSON response:
    """
    {"error": "Cannot complete this request. The requested order change is forbidden."}
    """
    Then order "1" is in state Paid
    Then customer id alice has paid orders worth 100 XTR
    Then customer id alice has current orders worth 65 XTR
    Then account for customer alice has a current balance of 0 XTR
