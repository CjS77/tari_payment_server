@order_with_name
Feature: Order id using order name
  Background:
    Given a server configuration
      | use_x_forwarded_for | true   |
      | order_id_field      | name |
    Given a blank slate

  Scenario: The server can be configured to user the name field to id orders.
    When Customer #1 ["alice"] places order with name "alice001" for 2400 XTR
    Then customer id 1 has current orders worth 2400 XTR
    And account for customer 1 has a current balance of 0 XTR
    And order "alice001" is in state Unclaimed

