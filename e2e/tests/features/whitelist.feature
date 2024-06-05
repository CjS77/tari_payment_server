Feature: Shopify webhook endpoints are whitelisted
  Background:
    Given a server configuration
      | use_x_forwarded_for | true |
      | use_forwarded       | true |
      | shopify_whitelist   | 10.0.0.5 |
    Given a blank slate

    Scenario: An unauthenticated user tries to access the webhook endpoint
        When I GETs to "/shopify/health" with body
        Then I receive a 403 Forbidden response
        Then I receive a partial JSON response:
        """
        {"error":"Authentication Error. Request was made from a forbidden peer"}
        """
