Feature: Admins can update the memo field of an order
  Background:
    Given a database with some accounts
    Given some role assignments

  Scenario: Unauthenticated users cannot access the `/order_memo` endpoint
    When User PATCHs to "/api/order_memo" with body
        """
        {
          "order_id": "1",
          "new_memo": "Ervin was here"
        }
        """
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user cannot access the `/order_memo` endpoint
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice PATCHs to "/api/order_memo" with body
        """
        {
          "order_id": "1",
          "new_memo": "Chancellor on brink of THIRD bailout for banks",
          "reason": "I want to update the memo"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Admin user cannot access the `/order_memo` endpoint if he does not ask for 'write' role
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin PATCHs to "/api/order_memo" with body
        """
        {
          "order_id": "1",
          "new_memo": "Chancellor on brink of THIRD bailout for banks"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: An admin can update the memo on behalf of a user
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin PATCHs to "/api/order_memo" with body
        """
        {
          "order_id": "1",
          "new_memo": "Threw in some purple jelly beans"
        }
        """
    Then I receive a 200 OK response
    Then I receive a partial JSON response:
    """
    {"order_id": "1", "memo": "Threw in some purple jelly beans"}
    """
    Then the OnOrderModified trigger fires with
    """
    {
      "old_order": {"order_id": "1", "memo": "Manually inserted by Keith"},
      "new_order": {"order_id": "1", "memo": "Threw in some purple jelly beans"}
    }
    """



