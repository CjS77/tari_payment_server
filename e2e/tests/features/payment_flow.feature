Feature: Order flow
  Background:
    Given a blank slate

  Scenario: Standard order flow
    When Alice places an order "alice001" on the store. Memo "": "Item A" for 100T, "Item B" for 200T
    Then Alice's account has a balance of 300 Tari
    Then Alice's order "alice001" is pending
    When Alice's sends a payment of 300 Tari
    Then order "alice001" is fulfilled


