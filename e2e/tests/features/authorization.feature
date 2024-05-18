Feature: users must supply an access token when accessing protected endpoints
  Background:
    Given a database with some accounts
    Given some role assignments

  Scenario: User tries to access a protected endpoint without an access token
    Given the user is not logged in
    When User GETs to "/api/check_token" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: User can access protected endpoints with a valid access token
    When Alice authenticates with nonce = 1 and roles = "user"
    Then I am logged in
    When Alice GETs to "/api/check_token" with body
    Then I receive a 200 OK response with the message 'Token is valid.'

  Scenario: User cannot modify the signature on their access token
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice modifies the signature on the access token to "invalid"
    When Alice GETs to "/api/check_token" with body
    Then I receive a 401 Forbidden response with the message 'An error occurred validating the jwt.'

  Scenario: User cannot modify any claims on their access token
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice modifies the roles on the access token to "super_admin"
    When Alice GETs to "/api/check_token" with body
    Then I receive a 401 Unauthorized response with the message 'An error occurred validating the jwt.'

  Scenario: User cannot access protected endpoints with an expired token
    When Alice authenticates with nonce = 1 and roles = "user"
    When the access token expires
    When Alice GETs to "/api/check_token" with body
    Then I receive a 401 Unauthorized response with the message 'token has expired'

  Scenario: User cannot sign their own access token
    When Alice creates a self-signed SuperAdmin access token
    When Alice GETs to "/api/check_token" with body
    Then I receive a 401 Unuathorized response with the message 'signature has failed verification'
