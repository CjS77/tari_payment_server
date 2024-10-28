@other_prices
Feature: Order Fulfillment using different price fields
  Background:
    Given a server configuration
      | use_x_forwarded_for          | true  |
      | disable_memo_signature_check | true  |
      | strict_mode                  | false |
      | price_field                  | subtotal |
    Given a blank slate
    Given an authorized wallet with secret df158b8389c68aac01a91276b742d2527f951d3c7289e4ccdecfa0672947270e
    """
      {
        "address": "14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
        "ip_address": "192.168.1.100"
      }
    """
    Given the exchange rate is 10 Tari per USD

  Scenario: When price field is line_items, paying the subtotal amount is sufficient
    When Customer #1 ["alice"] places order "1000" with price details:
     | total      | 300 | USD |
     | subtotal   | 240 | USD |
     | line_items | 230 | USD |
    Then customer id 1 has current orders worth 2400 XTR
    And account for customer 1 has a current balance of 0 XTR
    And order "1000" is in state Unclaimed
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {
      "payment": {
         "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
         "amount":2500000000,
         "txid":"payment001",
         "memo": "order 1000"
      },
      "auth": {"address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":1,"signature":"ac509cfdb1915e572faf9fb5c29ee5ab364f177f352f36a8f8b50ab48f34374961103694fea123f30a1e0ec66321186b6c3ead8e5cedf88c64e3256c1610fb03"}
    }
    """
    Then I receive a 200 Ok response with the message '"success":true'
    # Transaction is not confirmed yet
    Then order "1000" is in state New
    And account for customer 1 has a current balance of 0 XTR
    And User Alice has a pending balance of 2500 XTR
    When a confirmation arrives from x-forwarded-for 192.168.1.100
    """
    { "confirmation": {"txid": "payment001"},
      "auth": {
        "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
        "nonce":2,
        "signature":"16729e3c9b08022a16edfa0bf2e1cfc1d8dadd5e6387b4fd76005ac8a20c0f53087434524748981e9c61f9b7ce122fdf1a45299b132cd4495b19b7d1208cea01"
      }
    }
    """
    Then order "1000" is in state Paid
    And account for customer 1 has a current balance of 100 XTR
    And the OrderPaid trigger fires with
    """
    {
      "order": {
      "order_id": "1000",
      "alt_id": "#1000",
      "customer_id": "1",
      "total_price": 2400000000,
      "currency": "USD",
      "id": 1,
      "status": "Paid"
    }
   }
   """

  Scenario: When price field is subtotal, paying the line item amount is not sufficient
    When Customer #1 ["alice"] places order "1000" with price details:
      | total      | 300 | USD |
      | subtotal   | 260 | USD |
      | line_items | 240 | USD |
    Then customer id 1 has current orders worth 2600 XTR
    And account for customer 1 has a current balance of 0 XTR
    And order "1000" is in state Unclaimed
    When a payment arrives from x-forwarded-for 192.168.1.100
    """
    {
      "payment": {
         "sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
         "amount":2500000000,
         "txid":"payment001",
         "memo": "order 1000"
      },
      "auth": {"address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG","nonce":1,"signature":"ac509cfdb1915e572faf9fb5c29ee5ab364f177f352f36a8f8b50ab48f34374961103694fea123f30a1e0ec66321186b6c3ead8e5cedf88c64e3256c1610fb03"}
    }
    """
    Then I receive a 200 Ok response with the message '"success":true'
    # Transaction is not confirmed yet
    Then order "1000" is in state New
    And account for customer 1 has a current balance of 0 XTR
    And User Alice has a pending balance of 2500 XTR
    When a confirmation arrives from x-forwarded-for 192.168.1.100
    """
    { "confirmation": {"txid": "payment001"},
      "auth": {
        "address":"14z3iHvgokZcXmokAYQKveeJ4rMqSGtPahrC2CPvx63UQmG",
        "nonce":2,
        "signature":"16729e3c9b08022a16edfa0bf2e1cfc1d8dadd5e6387b4fd76005ac8a20c0f53087434524748981e9c61f9b7ce122fdf1a45299b132cd4495b19b7d1208cea01"
      }
    }
    """
    Then order "1000" is in state New
    And account for customer 1 has a current balance of 2500 XTR

