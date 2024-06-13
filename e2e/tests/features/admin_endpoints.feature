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

  Scenario: A random address has the default role
    Then address 3a5f3651ca97dc6dea42670d7a62dc746d6d4a6f72ef25d48ab36252d71a1627b6 has roles "user"

  Scenario: Roles change after Super_Admin edits them
    Given some role assignments
    When Super authenticates with nonce = 1
    Then address 98348e08c9076879bfe3e26f11fb7ef0391a2f4d3bf20dd58507ff90297f813529 has roles "user"
    When Super POSTs to "/api/roles" with body
        """
        [{
          "address": "98348e08c9076879bfe3e26f11fb7ef0391a2f4d3bf20dd58507ff90297f813529",
          "apply": ["write","read_all"]
        }]
        """
    Then address 98348e08c9076879bfe3e26f11fb7ef0391a2f4d3bf20dd58507ff90297f813529 has roles "user,write,read_all"
    When Super POSTs to "/api/roles" with body
        """
        [{
          "address": "98348e08c9076879bfe3e26f11fb7ef0391a2f4d3bf20dd58507ff90297f813529",
          "revoke": ["write"]
        }]
        """
    Then address 98348e08c9076879bfe3e26f11fb7ef0391a2f4d3bf20dd58507ff90297f813529 has roles "user,read_all"


  Scenario: You cannot revoke the default role
    Given some role assignments
    When Super authenticates with nonce = 1
    When Super POSTs to "/api/roles" with body
        """
        [{
          "address": "98348e08c9076879bfe3e26f11fb7ef0391a2f4d3bf20dd58507ff90297f813529",
          "revoke": ["user"]
        }]
        """
    Then address 98348e08c9076879bfe3e26f11fb7ef0391a2f4d3bf20dd58507ff90297f813529 has roles "user"

  Scenario: Non-super users have any subset of roles they are assigned
    Given some role assignments
    When Super authenticates with nonce = 1
    When Super POSTs to "/api/roles" with body
        """
        [{
          "address": "b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
          "apply": ["read_all", "write", "payment_wallet"]
        }]
        """
    Then address b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d has roles "read_all,write,payment_wallet,user"
    Then address b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d has roles "read_all,payment_wallet,user"
    Then address b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d has roles "payment_wallet"
