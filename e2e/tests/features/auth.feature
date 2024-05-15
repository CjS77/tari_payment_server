Feature: Users receive an access token when authenticating with a login token
  Background:
    Given a blank slate

  Scenario: User authenticates without a login token
    When I authenticate with the auth header
    Then I receive a 400 BadRequest response with the message '{"error":"Auth token signature invalid or not provided"}'


  Scenario: User authenticates with the wr0ng login token
    When I authenticate with the auth header
      """
      tpg_auth_token: some made up nonsense
      """
    Then I receive a 400 BadRequest response with the message '{"error":"Authentication Error. Login token is not in the correct format. InvalidTokenStructure"}'


  Scenario: User authenticates with an invalid signature
    When I authenticate with the auth header
      """
      tpg_auth_token: eyJhbGciOiJSaXN0cmV0dG8yNTYifQ.\
      eyJhZGRyZXNzIjp7Im5ldHdvcmsiOiJuZXh0bmV0IiwicHV\
      ibGljX2tleSI6IjEyYTI1MDRhNzhmMDg5MzBjMmQzMzU3MD\
      hmYWU4MDY5NmIyMTdkMjNiZDJkNDczZTEyN2Q4ZjVhMzBlM\
      jgxNjUifSwibm9uY2UiOjE3MTE0NDUxMTgsImRlc2lyZWRf\
      cm9sZXMiOlsidXNlciIsIndyaXRlIl19.\
      bad_sig_Uip03HFi5q65zE-QBq8iyEuT-IkLy9KeSHmB3UGkPIJXSDrKDVU_lg6JfBY4ch7BxwyH5iLDEiDzAQ
      """
    Then I receive a 401 Unauthorized response with the message 'Authentication Error. Login token signature is invalid. malformed token signature'

  Scenario: User authenticates with a valid token signature, but asks for roles they aren't entitled to
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user, read_all, write"
    Then I receive a 403 Forbidden response with the message 'Insufficient Permissions.'

  Scenario: User authenticates with a valid token
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user"
    Then I am logged in
    Then my access token starts with 'eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ'

  Scenario: User authenticates with a valid token, asks for a subset of roles
    Given some role assignments
    When Admin authenticates with nonce = 1 and roles = "user, read_all"
    Then I am logged in
    Then my access token starts with 'eyJhbGciOiJSaXN0cmV0dG8yNTYiLCJ0eXAiOiJKV1QifQ'

  Scenario: User authenticates with an invalid nonce
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user"
    Then I am logged in
    When Alice authenticates with nonce = 1 and roles = "user"
    Then I receive a 401 Unauthorized response with the message 'Nonce is not strictly increasing'

  Scenario: User can authenticate multiple times
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user"
    Then I am logged in
    When Alice authenticates with nonce = 2 and roles = "user"
    Then I am logged in



