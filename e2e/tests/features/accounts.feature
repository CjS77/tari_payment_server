@balances
Feature: Accounts endpoint
  Background:
    Given a database with some accounts

  Scenario: Unauthenticated user cannot access the `balance` endpoint
    When User GETs to "/api/balance" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user can access their own balance
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/balance" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "total_confirmed":0,"total_paid":0,
      "current_balance":0
    }
    """

  Scenario: Standard user cannot access another user's balance
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user"
    When Bob authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/balance/14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: User with ReadAll role can access another balance
    Given some role assignments
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin GETs to "/api/balance/14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address":"14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp",
      "total_confirmed":0,"total_paid":0,
      "current_balance":0
    }
    """

  Scenario: SuperAdmin role can access another balance
    Given some role assignments
    Given a super-admin user (Super)
    When Super authenticates with nonce = 1
    When Super GETs to "/api/balance/14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address":"14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp",
      "total_confirmed":0,"total_paid":0,
      "current_balance":0
    }
    """
