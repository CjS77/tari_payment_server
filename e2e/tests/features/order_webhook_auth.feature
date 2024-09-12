@order_flow
Feature: Order flow
  Background:
    Given a blank slate

  Scenario: Standard order flow
    When Customer #1 ["alice"] places order "alice001" for 2500 XTR, with memo
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "order_id":"alice001",
      "signature":"601d2f88738b9aacdf75b62de9db23bb8d1aeb9fef8c7d127a05e9648729e64143adcee0a3d50665eb9c7817a0035ad9ac6248a8547bd6d5a4796b917bd57d09"
    }
    """
    Then order "alice001" is in state New
    And customer id 1 has current orders worth 2500 XTR
    And account for customer alice has a current balance of 0 XTR

  Scenario: Replaying signature for different order fails
    When Customer #1 ["alice"] places order "alice002" for 2500 XTR, with memo
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "order_id":"alice002",
      "signature":"56e39d539f1865742b41993bdc771a2d0c16b35c83c57ca6173f8c1ced34140aeaf32bfdc0629e73f971344e7e45584cbbb778dc98564d0ec5c419e6f9ff5d06"
    }
    """
    # Webhook endpoints always return 200. Check the body for success status.
    Then I receive a 200 Ok response with the message '"success":true'
    And order "alice002" is in state Unclaimed

  Scenario: Replaying signature with different address fails
    When Customer #1 ["alice"] places order "alice001" for 2500 XTR, with memo
    """
    {
      "address":"14sa5AzjqqrzfiyqGkajoNcFrqkCK7syB4rvNNL65f2PjLD",
      "order_id":"alice001",
      "signature":"56e39d539f1865742b41993bdc771a2d0c16b35c83c57ca6173f8c1ced34140aeaf32bfdc0629e73f971344e7e45584cbbb778dc98564d0ec5c419e6f9ff5d06"
    }
    """
    # Webhook endpoints always return 200. Check the body for success status.
    Then I receive a 200 Ok response with the message '"success":true'
    And order "alice001" is in state Unclaimed



