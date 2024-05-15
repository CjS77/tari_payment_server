Feature: Accounts
  Background:
    Given a database with some accounts
    Given a super-admin user (Super)

  Scenario: Unuathenticated user cannot access the `account` endpoint
    When User GETs to "/api/account" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user can access their own account
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/account" with body
    Then I receive a 200 Ok response with the message 'xxx'
