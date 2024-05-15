Feature: super-admin
  Background:
    Given a database with some accounts
    Given a super-admin user (Super)

  Scenario: Unuathenticated user cannot access the `roles` endpoint
      When User POSTs to "/api/roles" with body
        """
        [{
          "address": "b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
          "apply": ["super_admin", "write"]
        }]
        """
      Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user cannot access the `roles` endpoint
    Given some role assignments
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice POSTs to "/api/roles" with body
        """
        [{
          "address": "b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
          "apply": ["super_admin", "write"]
        }]
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Super-admin can access the `roles` endpoint
    Given some role assignments
    When Super authenticates with nonce = 1
    When Super POSTs to "/api/roles" with body
        """
        [{
          "address": "b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
          "apply": ["read_all", "write"],
          "revoke": ["super_admin"]
        }]
        """
    Then I receive a 200 Ok response with the message ''
