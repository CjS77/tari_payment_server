Feature: super-admin
  Background:
    Given a database with some accounts
    Given a super-admin user (Super)

  Scenario: Unuathenticated user cannot access the `roles` endpoint
      When User POSTs to "/api/roles" with body
        """
        [{
          "address": "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
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
          "address": "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
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
          "address": "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
          "apply": ["read_all", "write"],
          "revoke": ["super_admin"]
        }]
        """
    Then I receive a 200 Ok response with the message ''

  Scenario: A random address has the default role
    Then address 142Eyn9FMCsBVRsFBc2zqfgBxPTTpX9dYjtrPABa9whREdia has roles "user"

  Scenario: Roles change after Super_Admin edits them
    Given some role assignments
    When Super authenticates with nonce = 1
    Then address 14sa5AzjqqrzfiyqGkajoNcFrqkCK7syB4rvNNL65f2PjLD has roles "user"
    When Super POSTs to "/api/roles" with body
        """
        [{
          "address": "14sa5AzjqqrzfiyqGkajoNcFrqkCK7syB4rvNNL65f2PjLD",
          "apply": ["write","read_all"]
        }]
        """
    Then address 14sa5AzjqqrzfiyqGkajoNcFrqkCK7syB4rvNNL65f2PjLD has roles "user,write,read_all"
    When Super POSTs to "/api/roles" with body
        """
        [{
          "address": "14sa5AzjqqrzfiyqGkajoNcFrqkCK7syB4rvNNL65f2PjLD",
          "revoke": ["write"]
        }]
        """
    Then address 14sa5AzjqqrzfiyqGkajoNcFrqkCK7syB4rvNNL65f2PjLD has roles "user,read_all"


  Scenario: You cannot revoke the default role
    Given some role assignments
    When Super authenticates with nonce = 1
    When Super POSTs to "/api/roles" with body
        """
        [{
          "address": "14sa5AzjqqrzfiyqGkajoNcFrqkCK7syB4rvNNL65f2PjLD",
          "revoke": ["user"]
        }]
        """
    Then address 14sa5AzjqqrzfiyqGkajoNcFrqkCK7syB4rvNNL65f2PjLD has roles "user"

  Scenario: Non-super users have any subset of roles they are assigned
    Given some role assignments
    When Super authenticates with nonce = 1
    When Super POSTs to "/api/roles" with body
        """
        [{
          "address": "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
          "apply": ["read_all", "write", "payment_wallet"]
        }]
        """
    Then address 14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt has roles "read_all,write,payment_wallet,user"
    Then address 14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt has roles "read_all,payment_wallet,user"
    Then address 14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt has roles "payment_wallet"
