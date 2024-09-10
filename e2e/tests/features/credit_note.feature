@credit_note
Feature: Admins can issue credit notes
  Background:
    Given a database with some accounts
    Given some role assignments

  Scenario: Unuathenticated users cannot access the `/credit` endpoint
    When User POSTs to "/api/credit" with body
        """
        {
          "customer_id": "eric101",
          "amount": 1000000000,
          "reason": "For being awesome"
        }
        """
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user cannot access the `/credit` endpoint
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice POSTs to "/api/credit" with body
        """
        {
          "customer_id": "alice101",
          "amount": 1000000000,
          "reason": "I like free stuff"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Admin user cannot access the `/credit` endpoint if he does not ask for 'write' role
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin POSTs to "/api/credit" with body
        """
        {
          "customer_id": "alice101",
          "amount": 1000000000,
          "reason": "Refund"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: An admin can issue a credit note for a customer id that has no account or orders and a subsequent order will be filled
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin POSTs to "/api/credit" with body
        """
        {
          "customer_id": "1024",
          "amount": 1000000000,
          "reason": "Grand prize winner"
        }
        """
    Then I receive a 200 OK response with the message "[]"
    Then account for customer 1024 has a current balance of 1000 XTR
    # Eric places an order from his wallet with these credentials:
    # Secret key: 9b72bd6f55466f693c71f4a5abeb0767c4c080cc4752d336b6c0381e2dee5b01
    # Public key: ecf774d7f185295b9c2b1f87072919d4e5bd1e280697b172a0db91f7caebd364
    # Address: ecf774d7f185295b9c2b1f87072919d4e5bd1e280697b172a0db91f7caebd36418
    When Customer #1024 ["eric101"] places order "order1024:1" for 250 XTR, with memo
    Then order "order1024:1" is in state Paid
    Then account for customer 1024 has a current balance of 750 XTR

  Scenario: An admin can issue a credit note for a customer id that has a pending order, and the order will be filled
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin POSTs to "/api/credit" with body
    """
    {
      "customer_id": "bob",
      "amount": 200000000,
      "reason": "Helping a buddy out"
    }
    """
    Then I receive a 200 OK response
    Then I receive a partial JSON response:
    """
    [{"order_id": "2", "customer_id":"bob", "memo": "Manually inserted by Charlie", "total_price": 200000000}]
    """
    Then account for customer bob has a current balance of 0 XTR

  Scenario: A super admin can issue a credit note for a customer id
    Given a super-admin user (Super)
    When Super authenticates with nonce = 1
    When Super POSTs to "/api/credit" with body
    """
    {
      "customer_id": "bob",
      "amount": 250000000,
      "reason": "Because I can"
    }
    """
    Then I receive a 200 OK response
    Then I receive a partial JSON response:
    """
    [{"order_id": "2", "customer_id":"bob", "memo": "Manually inserted by Charlie", "total_price": 200000000}]
    """
    Then account for customer bob has a current balance of 50 XTR

Scenario: An admin can remove funds by issuing a negative credit note
  When Admin authenticates with nonce = 1 and roles = "write"
  When Admin POSTs to "/api/credit" with body
        """
        {
          "customer_id": "eric101",
          "amount": 1000000000,
          "reason": "Grand prize winner"
        }
        """
  Then I receive a 200 OK response with the message "[]"
  Then account for customer eric101 has a current balance of 1000 XTR
  When Admin POSTs to "/api/credit" with body
        """
        {
          "customer_id": "eric101",
          "amount": -900000000,
          "reason": "Fix fat finger error"
        }
        """
  Then I receive a 200 OK response with the message "[]"
  Then account for customer eric101 has a current balance of 100 XTR
  When Admin POSTs to "/api/credit" with body
        """
        {
          "customer_id": "eric101",
          "amount": -500000000,
          "reason": "Fine!r"
        }
        """
  Then I receive a 200 OK response with the message "[]"
  Then account for customer eric101 has a current balance of -400 XTR
