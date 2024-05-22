Feature: Order flow
  Background:
    Given a blank slate

  Scenario: Standard order flow
    When Customer #1 ["alice"] places order "alice001" for 2500 XTR, with memo
    """
    {
      "address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
      "order_id":"alice001",
      "signature":"56e39d539f1865742b41993bdc771a2d0c16b35c83c57ca6173f8c1ced34140aeaf32bfdc0629e73f971344e7e45584cbbb778dc98564d0ec5c419e6f9ff5d06"
    }
    """
    Then Customer #1 has current orders worth 2500 XTR
    And Alice has a balance of 0 Tari
    Then order "alice001" is in state New
    When Alice sends a payment of 2525 Tari
    Then order "alice001" is fulfilled
    And Alice has a balance of 25 Tari

#  Scenario: Replaying signature for different order fails
#    When Customer #1 ["alice"] places order "alice001" for 2500 XTR, with memo
#    """
#    {
#      "address":"b8971598a865b25b6508d4ba154db228e044f367bd9a1ef50dd4051db42b63143d",
#      "order_id":"alice002",
#      "signature":"56e39d539f1865742b41993bdc771a2d0c16b35c83c57ca6173f8c1ced34140aeaf32bfdc0629e73f971344e7e45584cbbb778dc98564d0ec5c419e6f9ff5d06"
#    }
#    """
#    Then I receive a 202 Accepted response with the message 'does not match order ID'
#
#  Scenario: Replaying signature with different address fails
#    When Customer #1 ["alice"] places order "alice001" for 2500 XTR, with memo
#    """
#    {
#      "address":"680ac255be13e424dd305c2ed93f58aee73670fadb97d733ad627efc9bb165510b",
#      "order_id":"alice001",
#      "signature":"56e39d539f1865742b41993bdc771a2d0c16b35c83c57ca6173f8c1ced34140aeaf32bfdc0629e73f971344e7e45584cbbb778dc98564d0ec5c419e6f9ff5d06"
#    }
#    """
#    Then I receive a 202 Accepted response with the message 'signature was invalid'



