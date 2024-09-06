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
      "address": "14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "signature": "foobar"
    }
    """
    Then I receive a 400 BadRequest response with the message 'Invalid signature'

  # ----------------------------- Tari Address -----------------------------
  # Network: mainnet
  # Secret key: bf9da371add03729f9df1ab8f6356a7104868fe75de9d5235d33af28cfcae701
  # Public key: 10c2e78e1bbbe58779eade51cf6d66f26288d934e9eda26fb20c8bcb88825d2a
  # Address      : 145yntp6GHTuDuTVPtbyPdo8rVjRppjdfyoxZeY958kh1Mn
  #------------------------------------------------------------------------
  Scenario: An unauthenticated user can claim an order with the correct signature, even if they already have a previous account
    When Customer #142 ["anon@example.com"] places order "order10256" for 2400 XTR, with memo
    When User POSTs to "/order/claim" with body
    """
    {
      "address":"145yntp6GHTuDuTVPtbyPdo8rVjRppjdfyoxZeY958kh1Mn",
      "order_id":"order10256",
      "signature":"6689435195fda30c9c3805b844b45adadac2d5b5ca647dca342531e50bd5ff658b0aa458d184e720a6fcd5c7be4efec65e7e7e78e1c86003bf00147962cb540d"}
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
      "claimant": "145yntp6GHTuDuTVPtbyPdo8rVjRppjdfyoxZeY958kh1Mn"
    }
    """

  # ----------------------------- Memo Signature -----------------------------
  # Wallet address: 14NPqUxFyJwbZ6wJ8hpuTuX5oWbQt7XeMJWXMZdMSiA19Fj
  # Public key    : 480487461530011b99563cb69a96f11719ddf08307459aee4d0cab15090bda7a
  # emoji id      : 🎢🌋🐲🎠🍄🍬🌂🍊👕🎳🍼💕👖👒🚒🍆🍈🔭🚑🐭🌝🎓👖🚂🎨🌴💀🍄🌟🌰🔪🐜🐳
  # Secret        : ac8bac66b8efc8f2055ba8dcd95ad5f7f9e791d876a50642d563426fc86de109
  # Network       : mainnet
  # auth:  {"address":"14NPqUxFyJwbZ6wJ8hpuTuX5oWbQt7XeMJWXMZdMSiA19Fj","order_id":"anon100","signature":"be215f32c4b7a9a7a4da600a53bfdc7ca735e64cf5f0ecac2b5904ac24a1fb04376c2c7f3d96ca95e0e16a124f80dc846702e14df6387ad744ad5737f14fa90c"}
  # ------------------------------------------------------------------------
  Scenario: Previously unlinked accounts can claim an order with the correct signature, and the order can be paid.
    Given a payment of 100 XTR from address 14NPqUxFyJwbZ6wJ8hpuTuX5oWbQt7XeMJWXMZdMSiA19Fj in tx tx4804forAnon100
    When Customer #999 ["who@example.com"] places order "anon100" for 60 XTR, with memo
    When User POSTs to "/order/claim" with body
    """
    {
       "address":"14NPqUxFyJwbZ6wJ8hpuTuX5oWbQt7XeMJWXMZdMSiA19Fj",
       "order_id":"anon100",
       "signature":"72dcd2ff713f5f38c45f0f87426646eeac49a360613bf9d7195ce009dd23df392b908baa26206c129b975caa382cc0bad603f13e0c8f0cbf49809c3546a10105"
    }
    """
    Then I receive a 200 Ok response
    Then the OrderClaimed trigger fires with
    """
    {
      "order": { "order_id": "anon100", "total_price": 60000000, "status": "New" },
      "claimant": "14NPqUxFyJwbZ6wJ8hpuTuX5oWbQt7XeMJWXMZdMSiA19Fj"
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
       "address":"14NPqUxFyJwbZ6wJ8hpuTuX5oWbQt7XeMJWXMZdMSiA19Fj",
       "order_id":"anon101",
       "signature":"e88a14e2e1de92bc68cf982c0cfc9c3c39c3f4a7f712b6b0294961da6b9f70059f9025d635e3221e1b7c79ee33665881f8320243ae4a59b9226b45f86eb63d05"
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
      "claimant": "14NPqUxFyJwbZ6wJ8hpuTuX5oWbQt7XeMJWXMZdMSiA19Fj"
    }
    """

  Scenario: Claiming an order before making the order fails
    When User POSTs to "/order/claim" with body
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "order_id":"does-not-exist",
      "signature":"1e06e8f1de61644ba380c8d82a124e0b288b86501b6283e93bc8094ddd6d980765c4edc2b73dca69847cb9f15e3685f67ed4431ee527e9ef63ae176d0c5f2a09"
    }
    """
    Then I receive a 404 NotFound response with the message 'The requested order does-not-exist does not exist'
