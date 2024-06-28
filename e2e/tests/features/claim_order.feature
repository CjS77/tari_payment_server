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

  Scenario: An unauthenticated user can claim an order with the correct signature, even if they already have a previous account
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
    { "order_id": "order10256", "total_price": 2400000000 }
    """
    Then the OrderClaimed trigger fires with
    """
    {
      "order": { "order_id": "order10256", "total_price": 2400000000, "status": "New" },
      "claimant": "680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b"
    }
    """

  # ----------------------------- Memo Signature -----------------------------
  # Wallet address: 480487461530011b99563cb69a96f11719ddf08307459aee4d0cab15090bda7a88
  # Public key    : 480487461530011b99563cb69a96f11719ddf08307459aee4d0cab15090bda7a
  # emoji id      : 🎢🌋🐲🎠🍄🍬🌂🍊👕🎳🍼💕👖👒🚒🍆🍈🔭🚑🐭🌝🎓👖🚂🎨🌴💀🍄🌟🌰🔪🐜🐳
  # Secret        : ac8bac66b8efc8f2055ba8dcd95ad5f7f9e791d876a50642d563426fc86de109
  # Network       : mainnet
  # auth: {"address":"480487461530011b99563cb69a96f11719ddf08307459aee4d0cab15090bda7a88","order_id":"anon101","signature":"2ee4933f2a845e424a784d0c35d84bfb2836191aa5ffc4adc20de3b733da8478c0a0afcf941ed8a26d195eb1f748d2101ed6434af49823994334778385fc0b04"}
  # ------------------------------------------------------------------------
  Scenario: Previously unlinked accounts can claim an order with the correct signature, and the order can be paid.
    Given a payment of 100 XTR from address 480487461530011b99563cb69a96f11719ddf08307459aee4d0cab15090bda7a88 in tx tx4804forAnon100
    When Customer #999 ["who@example.com"] places order "anon100" for 60 XTR, with memo
    When User POSTs to "/order/claim" with body
    """
    {
       "address":"480487461530011b99563cb69a96f11719ddf08307459aee4d0cab15090bda7a88",
       "order_id":"anon100",
       "signature":"1295995d5698365ecc82ee7acf4acd11dfe678a9b860fd2054d1d3b5b9c14951daa3a09dd00fa9869fedf5e321b877a22ad3cc7291e93d3d320e2d4ac8134d05"
    }
    """
    Then I receive a 200 Ok response
    Then the OrderClaimed trigger fires with
    """
    {
      "order": { "order_id": "anon100", "total_price": 60000000, "status": "New" },
      "claimant": "480487461530011b99563cb69a96f11719ddf08307459aee4d0cab15090bda7a88"
    }
    """
    Then the OrderPaid trigger fires with
    """
    {
      "order": { "order_id": "anon100", "total_price": 60000000, "status": "Paid" }
    }
    """


  Scenario: New addresses can claim an order with the correct signature.
    When Customer #999 ["who@example.com"] places order "anon101" for 28 XTR, with memo
    When User POSTs to "/order/claim" with body
    """
    {
       "address":"480487461530011b99563cb69a96f11719ddf08307459aee4d0cab15090bda7a88",
       "order_id":"anon101",
       "signature":"2ee4933f2a845e424a784d0c35d84bfb2836191aa5ffc4adc20de3b733da8478c0a0afcf941ed8a26d195eb1f748d2101ed6434af49823994334778385fc0b04"
    }
    """
    Then I receive a 200 Ok response
    And I receive a partial JSON response:
    """
    { "order_id": "anon101", "total_price": 28000000 }
    """
    Then the OrderClaimed trigger fires with
    """
    {
      "order": { "order_id": "anon101", "total_price": 28000000, "status": "New" },
      "claimant": "480487461530011b99563cb69a96f11719ddf08307459aee4d0cab15090bda7a88"
    }
    """
