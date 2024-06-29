@expiry
Feature: Expire old orders
  Background:
    # For testing, the expiry limits are 1s for unclaimed and 2s for unpaid


  Scenario: Expire unclaimed orders
    Given a blank slate
    When Customer #1 ["Alex"] places order "order1" for 1 XTR, with memo
    Then order "order1" is in state Unclaimed
    And account for 1 has current orders worth 1 XTR
    Then pause for 2000 ms
    When I expire old orders
    Then order "order1" is in state Expired
    And account for 1 has current orders worth 0 XTR


  Scenario: Expire a mix of orders
    Given a database with some accounts
    Then account for alice has current orders worth 165 XTR
    When Customer #1 ["Alex"] places order "order1" for 1 XTR, with memo
    Then order "order1" is in state Unclaimed
    Then order "1" is in state New
    Then pause for 1000 ms
    When Customer #2 ["Barb"] places order "order2" for 1 XTR, with memo
    Then pause for 800 ms
    When I expire old orders
    Then order "order1" is in state Expired
    Then order "order2" is in state Unclaimed
    Then order "1" is in state New
    Then pause for 1100 ms
    When I expire old orders
    Then order "order2" is in state Expired
    Then order "1" is in state Expired
    # All Alice's orders have expired
    And account for alice has current orders worth 0 XTR
