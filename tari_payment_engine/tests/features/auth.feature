Feature: Login and authentication
  Background:
    Given a database with some accounts

    Scenario: User logs in with correct credentials
      When I receive a valid login token for address = 'xxxxx', nonce = 1, and roles = [user, read_all]
      Then the user receives an auth token
      And the auth token is signed by the signer

    Scenario: User tries to reuse an old auth token (nonce checking)

    Scenario: User logs in with incorrect roles

    Scenario: User tries to use in with an expired access token

    Scenario: User can request account using auth token
        Given the user has a valid auth token

    Scenario: User can request orders using auth token




