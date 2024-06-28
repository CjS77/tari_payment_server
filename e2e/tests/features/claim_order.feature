@claiming
Feature: Order claiming
  Background:
    Given a database with some accounts
    Given some role assignments

  Scenario: An authenticated user cannot claim an order without a signature
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice POSTs to "/order/claim" with body
    """
    {
      "orderId": "1",
      "address": "aa3c076152c1ae44ae86585eeba1d348badb845d1cab5ef12db98fafb4fea55d6c",
      "signature": "foobar"
    }
    """
    Then I receive a 400 BadRequest response with the message 'Invalid signature'

  Scenario: An unauthenticated user can claim an order with the correct signature
    When Customer #142 ["anon@example.com"] places order "order10256" for 2400 XTR, with memo
    When User POSTs to "/order/claim" with body
    """
    {
      "address":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
      "order_id":"order10256",
      "signature":"70cff86acc645b460cb4f1e0ffecfa11871d5d50ea349774964e851a451f3202ee4a39b038ddce19cb88b6ef57259bd9f6aa26cf2594f5b04ec14db05320f900"}
    """
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    { "foo": "bar" }
    """
    Then the OrderClaimed trigger fires with
    """
    {

    }
    """
