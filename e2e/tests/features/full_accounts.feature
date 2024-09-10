@full_accounts
Feature: Full accounts endpoint (/api/history)
  Background:
    Given a database with some accounts
    Given some payments are received
    Given some role assignments


  Scenario: Unauthenticated user cannot access the `history` endpoint
    When User GETs to "/api/history" with body
    Then I receive a 401 Unauthenticated response with the message 'An error occurred, no cookie containing a jwt was found in the request.'

  Scenario: Standard user can access their own account history
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/history" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
      "balance":{
        "address":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt",
        "total_confirmed":0,
        "total_paid":0,
        "current_balance":0
      },
      "orders":[
        {"id":1,"order_id":"1","customer_id":"alice","memo":"Manually inserted by Keith","total_price":100000000,"status":"New"},
        {"id":3,"order_id":"3","customer_id":"alice","memo":"Manually inserted by Sam","total_price":65000000,"status":"New"}
      ],
      "payments":[
        {"txid":"alicepayment001","sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt","amount":15000000,"status":"received"},
        {"txid":"alicepayment002","sender":"14wqR3rjyVbjgXDyLVaL97p3CksHc84cz9hLLMMTMYDjtBt","amount":100000000,"status":"received"}
      ],
      "settlements":[]
    }
    """

  Scenario: Standard user cannot access another user's account history with an address
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/history/address/14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: Standard user cannot access another user's account history with an account id
    When Alice authenticates with nonce = 1 and roles = "user"
    When Alice GETs to "/api/history/customer/2" with body
    Then I receive a 403 Forbidden response with the message 'Insufficient permissions.'

  Scenario: User with ReadAll role can access any account history with an address
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin GETs to "/api/history/address/14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address": "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp",
      "balance": {
        "address": "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp",
        "total_confirmed": 0,
        "total_paid": 0,
        "current_balance": 0
      },
      "orders": [
        {"id": 2, "order_id": "2", "customer_id": "bob", "memo": "Manually inserted by Charlie", "total_price": 200000000, "status": "New"},
        {"id": 4, "order_id": "4", "customer_id": "bob", "memo": "Manually inserted by Ray", "total_price": 350000000, "status": "New"}
      ],
      "payments": [
        {"txid": "bobpayment001", "sender": "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp", "amount": 50000000, "status": "received"},
        {"txid": "bobpayment002", "sender": "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp", "amount": 500000000, "status": "received"}
      ],
      "settlements": []
    }
    """

  Scenario: User with ReadAll role can access any account history with an address
    When Admin authenticates with nonce = 1 and roles = "user,read_all"
    When Admin GETs to "/api/history/customer/bob" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "customer_id": "bob",
      "balance": {
        "total_confirmed": 0,
        "total_paid": 0,
        "current_balance": 0,
        "addresses": []
      },
      "order_balance": {
        "customer_id": "bob",
        "total_current": 550000000,
        "total_paid": 0,
        "total_expired": 0,
        "total_cancelled": 0
      },
      "orders": [
        {"id": 2, "customer_id": "bob", "total_price": 200000000},
        {"id": 4, "customer_id": "bob", "total_price": 350000000}
      ],
      "settlements": []
    }
    """

  Scenario: SuperAdmin role can access another account
    Given a super-admin user (Super)
    When Super authenticates with nonce = 1
    When Super GETs to "/api/history/address/14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp" with body
    Then I receive a 200 Ok response
    Then I receive a partial JSON response:
    """
    {
      "address": "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp",
      "balance": {
        "address": "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp",
        "total_confirmed": 0,
        "total_paid": 0,
        "current_balance": 0
      },
      "orders": [
        {"id": 2, "order_id": "2", "customer_id": "bob", "total_price": 200000000, "status": "New"},
        {"id": 4, "order_id": "4", "customer_id": "bob", "total_price": 350000000, "status": "New"}
      ],
      "payments": [
        {"txid": "bobpayment001", "sender": "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp", "amount": 50000000},
        {"txid": "bobpayment002", "sender": "14XubwVbMhtp18SHrjfVKk7TRCx2yk7gZBbsjTPRWCXkCEp", "amount": 500000000}
      ]
    }
    """
