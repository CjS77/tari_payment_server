# This feature is temporarily disabled
Feature: Admins can manually attach an order to a wallet address
  Background:
    Given a database with some accounts
    Given some role assignments

  Scenario: Unauthenticated users cannot access the `/attach_order` endpoint
    When User POSTs to "/api/attach_order" with body
        """
        {
          "order_id": "1",
          "address": "680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
          "reason": "for the lulz"
        }
        """
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user cannot access the `/attach_order` endpoint
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice POSTs to "/api/attach_order" with body
        """
        {
          "order_id": "1",
          "address": "680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
          "reason": "for the lulz"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Admin user cannot access the `/attach_order` endpoint if he does not ask for 'write' role
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin POSTs to "/api/attach_order" with body
        """
        {
          "order_id": "1",
          "address": "680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
          "reason": "You must ask for the WRITE roles, geddit?"
        }
        """
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: The Tari address must be valid
    When Admin authenticates with nonce = 1 and roles = "user,read_all,write"
    When Admin POSTs to "/api/attach_order" with body
        """
        {
          "order_id": "1",
          "address": "deadbeefbe13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
          "reason": "That's not a Tari address!"
        }
        """
    Then I receive a 400 BadRequest response with the message 'Cannot recover public key'

  Scenario: An admin can attach an order to a new address.
    When Admin authenticates with nonce = 1 and roles = "user,read_all,write"
    When Admin POSTs to "/api/attach_order" with body
    """
    {
          "order_id": "1",
          "address": "4c827cbd0b9ba38a8659f05969c28d1020d096ba06aed8419fc25c13f1ea9a6e48",
          "reason": "User moved to new wallet"
    }
    """
    Then I receive a 400 BadRequest response with the message 'This feature is not supported yet'
