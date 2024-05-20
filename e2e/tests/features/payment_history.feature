Feature: Payment history endpoint
  Background:
    Given a database with some accounts
    Given some role assignments
    Given some payments are received

  Scenario: Unauthenticated user cannot access the `payments` endpoint
    When User GETs to "/api/payments" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: A user with an expired token cannot access the `payments` endpoint
    When Alice authenticates with nonce = 1 and roles = "user"
    When the access token expires
    When Alice GETs to "/api/payments" with body
    Then I receive a 401 Unauthenticated response with the message 'token has expired'

  Scenario: A user with an invalid token cannot access the `payments` endpoint
    When Alice creates a self-signed SuperAdmin access token
    When Alice GETs to "/api/payments" with body
    Then I receive a 401 Unauthenticated response with the message 'signature has failed verification'

  Scenario: Authenticated user can see their own payment history
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/payments" with body
    Then I receive a 200 OK response
    And I receive a partial JSON response:
    """
    {
      "address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "total_payments":115000000,
      "payments":[
        {"txid":"alicepayment001",
         "sender":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
         "amount":15000000
        },
        {"txid":"alicepayment002",
        "sender":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
        "amount":100000000
        }
      ]
    }
    """

  Scenario: Authenticated user cannot see another user's payment history
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/payments/680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: User with ReadAll Role can see another user's payment history
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin GETs to "/api/payments/680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b" with body
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    {
      "address":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
      "total_payments":550000000,
      "payments":[
        {"txid":"bobpayment001",
         "sender":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
         "amount":50000000
        },
        {"txid":"bobpayment002",
        "sender":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
        "amount":500000000
        }
      ]
    }
    """

  Scenario: SuperAdmin can see another user's payment history
    Given a super-admin user (Super)
    When Super authenticates with nonce = 1
    When Super GETs to "/api/payments/680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
      "total_payments":550000000,
      "payments":[
        {"txid":"bobpayment001",
         "sender":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
         "amount":50000000
        },
        {"txid":"bobpayment002",
        "sender":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
        "amount":500000000
        }
      ]
    }
    """
