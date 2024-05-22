Feature: Order flow
  Background:
    Given a blank slate

  Scenario: Standard order flow
    When Customer #1 ["alice"] places order "alice001" for 2500 XTR, with memo
    """
    { "address": "b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "order_id": "alice001",
      "signature": "deadbeef34534534534534543435345"
    }
    """
    Then Customer #1 has a balance of 2500 Tari
    Then order "alice001" is in state pending
    When Alice sends a payment of 2525 Tari
    Then order "alice001" is fulfilled
    And Alice has a balance of 25 Tari


