@wallet_management
Feature: Wallet Management
  Background:
    Given a blank slate
    Given some role assignments
    Given an authorized wallet with secret df158b8389c68aac01a91276b742d2527f951d3c7289e4ccdecfa0672947270e
    """
      {
        "address": "c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495",
        "ip_address": "192.168.1.100"
      }
    """

  Scenario: Unauthenticated user can list authorized addresses
    When User GETs to "/wallet/send_to" with body
    Then I receive a 200 OK response
    And I receive a partial JSON response:
    """
    ["c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495"]
    """

  Scenario: SuperAdmin can add an authorized wallet
    Given a super-admin user (Super)
    When Super authenticates with nonce = 1
    When Super POSTs to "/api/wallets" with body
    """
    {
      "address": "7a83ef47b358bead8f2e595814146784014002a6aa124c9b31134e489d27617309",
      "ip_address": "100.50.60.70",
      "initial_nonce": 1
    }
    """
    Then I receive a 200 OK response

  Scenario: Unauthenticated user cannot add an authorized wallet
    When User POSTs to "/api/wallets" with body
    """
    {
      "address": "7a83ef47b358bead8f2e595814146784014002a6aa124c9b31134e489d27617309",
      "ip_address": "100.50.60.70",
      "initial_nonce": 1
    }
    """
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Normal user cannot add an authorized wallet
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice POSTs to "/api/wallets" with body
    """
    {
      "address": "7a83ef47b358bead8f2e595814146784014002a6aa124c9b31134e489d27617309",
      "ip_address": "100.50.60.70",
      "initial_nonce": 1
    }
    """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: ReadAll admin cannot add an authorized wallet
    When Admin authenticates with nonce = 1 and roles = "read_all"
    When Admin POSTs to "/api/wallets" with body
    """
    {
      "address": "7a83ef47b358bead8f2e595814146784014002a6aa124c9b31134e489d27617309",
      "ip_address": "100.50.60.70",
      "initial_nonce": 1
    }
    """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Write admin cannot add an authorized wallet
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin POSTs to "/api/wallets" with body
    """
    {
      "address": "7a83ef47b358bead8f2e595814146784014002a6aa124c9b31134e489d27617309",
      "ip_address": "100.50.60.70",
      "initial_nonce": 1
    }
    """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: SuperAdmin user can list authorized wallets
    Given a super-admin user (Super)
    When Super authenticates with nonce = 1
    When Super GETs to "/api/wallets" with body
    Then I receive a 200 OK response
    And I receive a partial JSON response:
    """
    [
      {
        "address": "c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495",
        "ip_address": "192.168.1.100",
        "last_nonce": 0
      }
    ]
    """

  Scenario: Unauthenticated user cannot list authorized wallets
    When User GETs to "/api/wallets" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Normal user cannot list authorized wallets
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/wallets" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: ReadAll user can list authorized wallets
    When Admin authenticates with nonce = 1 and roles = "read_all"
    When Admin GETs to "/api/wallets" with body
    Then I receive a 200 OK response
    And I receive a partial JSON response:
    """
    [
      {
        "address": "c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495",
        "ip_address": "192.168.1.100",
        "last_nonce": 0
      }
    ]
    """

  Scenario: SuperAdmin user can remove an authorized addresses
    Given a super-admin user (Super)
    When Super authenticates with nonce = 1
    When Super DELETEs to "/api/wallets/c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495" with body
    Then I receive a 200 OK response
    When User GETs to "/wallet/send_to" with body
    Then I receive a 200 OK response
    And I receive a partial JSON response:
    """
    []
    """

  Scenario: Unauthenticated user cannot remove an authorized addresses
    When User DELETEs to "/api/wallets/c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Normal user cannot remove an authorized addresses
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice DELETEs to "/api/wallets/c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: ReadAll admin cannot remove an authorized addresses
    When Admin authenticates with nonce = 1 and roles = "read_all"
    When Admin DELETEs to "/api/wallets/c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Write admin cannot remove an authorized addresses
    When Admin authenticates with nonce = 1 and roles = "write"
    When Admin DELETEs to "/api/wallets/c009584dac6ad9ca0964e3dc93892c607ca37e049b4c30637fa477d0d601174495" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'








