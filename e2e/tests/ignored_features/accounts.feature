Feature: Accounts endpoint
  Background:
    Given a database with some accounts

  Scenario: Unauthenticated user cannot access the `account` endpoint
    When User GETs to "/api/account" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user can access their own account
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/account" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "id": 1,
      "total_received":0,
      "current_pending":0,
      "current_balance":0,
      "total_orders":165000000,
      "current_orders":165000000
    }
    """

    Scenario: Standard user cannot access another user's account
      Given some role assignments
      When Alice authenticates with nonce = 1 and roles = "user"
      When Bob authenticates with nonce = 1 and roles = "user"
      When Alice GETs to "/api/account/680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b" with body
      Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: User with ReadAll role can access another account
    Given some role assignments
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin GETs to "/api/account/680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "id": 2,
      "total_received":0,
      "current_pending":0,
      "current_balance":0,
      "total_orders":550000000,
      "current_orders":550000000
    }
    """

  Scenario: SuperAdmin role can access another account
    Given some role assignments
    Given a super-admin user (Super)
    When Super authenticates with nonce = 1
    When Super GETs to "/api/account/680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "id": 2,
      "total_received":0,
      "current_pending":0,
      "current_balance":0,
      "current_orders":550000000,
      "total_orders":550000000
    }
    """
