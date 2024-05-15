Feature: super-admin
  Background:
    Given a database with some accounts
    Given a super-admin user (Super)

  Scenario: Unuathenticated user cannot access the `roles` endpoint
      When User posts to /roles/b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d with body
        """
        ["super_admin", "write"]
        """
      Then I receive a 403 Forbidden response with the message 'Authentication Error. Insufficient Permissions.'

  Scenario: Standard user cannot access the `roles` endpoint
    When Alice authenticates with nonce = 1 and roles = "user"
    When User posts to /roles/b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d with body
        """
        ["super_admin", "write"]
        """
    Then I receive a 403 Forbidden response with the message 'Authentication Error. Insufficient Permissions.'
