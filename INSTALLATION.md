# Installation Guide

This guide provides a walkthrough of installing and setting up the Tari Payment Gateway with:
* A Shopify storefront
* A Tari console wallet.

## Tari Payment Server

## Shopify Storefront

### Initial setup

1. You need to have an existing Shopify store. If you don't have one, you can create one [here](https://www.shopify.com/).
2. Set up your store with the products you want to sell, configure the theme, etc.
3. If you only want to accept Tari payments, you can remove all other payment methods from your store. 
   To do this, go to `Apps and Sales Channels` and remove all pre-installed apps, except for "Online Store".
4. Configure your store to process **Manual Payments** ([Shopify Help](https://help.shopify.com/en/manual/payments/additional-payment-methods/activate-payment-methods)). 
   * To do this, go to `Settings` -> `Payments`. Under `Payment Capture Methods`, select `Manually`.
   * Under `Manual Payment Methods`, select `Create custom payment method`.
   * Enter the name of the payment method as `Tari Payment Server`.
     * Click `Activate`.

### Enable custom App development

1. From your store's admin, go to `Settings` -> `Apps and sales channels`.
2. Click on `Develop apps`.
3. Click on `Create app`.
4. Fill in the details of your app. Give the title `Tari integration`.
5. Select `Storefront API` and enable the following permissions:
   * Checkouts: `unauthenticated_write_checkouts`, `unauthenticated_read_checkouts`
   * Products: `unauthenticated_read_product_listings`, `unauthenticated_read_product_inventory`
   * Meta Objects: `unauthenticated_read_metaobjects`
6. Select `Admin API` and enable the following permissions:
   * Orders: `read_orders`, `write_orders`
   * Products: `write_products`, `read_products`
   * Shopify markets: `read_markets`
   * Meta Objects: `read_metaobject_definitions`, `read_metaobjects`, `write_metaobjects` 
7. Save the scopes.
8. Select the `API credentials` tab, and store the API key and API secret key in a safe place. You'll also need to
   fill in these values in your `.env` file for the Tari Payment Server.
9. Click `Install app` and follow the prompts to install the app in your store.
   * Save your Admin API token. **Warning**: You only get to see this once!
   * Save your Storefront API token.

### Configure the store to display prices in Tari.

Shopify makes it difficult to display prices in a currency other than the store's base currency, or in a 
'recognised' local currency. And in 2024, Shopify has been deprecating APIs that allows apps to alter the 
checkout experience, making it nearly impossible to provide a completely seamless experience for merchants wanting 
to provide an end-to-end Tari payment experience.

Tari Payment Server assumes that your store is using USD as a base currency, and uses the 
configured XTR-USD exchange rate to set a [metafield] on every product [variant] in the store every time the exchange 
rate is updated, as well as a global rate set in a metaobject variable in the odd instance where a product or 
variant price has not been set yet. 

**Important**: The source of truth for the exchange rate is the value in the Tari Payment Server database, 
not the store. If you update the exchange rate in the store, or alter the Tari price in the metafield for a 
product, then any orders using those products will display the wrong price, and you will get a mismatch between 
the amount the payment server is expecting and the final order price on the customer's invoice. You can use the 
CLI utility to update the exchange rate; this will reset all the [metafield] values in the shopify product list. 
You can freely update the USD price of products in the store. The payment server will detect this update and 
update the [metafield] value for the product automatically.

**Note**: The base unit for Tari is  _microTari (μT)_. The `tari_price` metafield always represents the price in μT, 
and therefore any representation of the price in XTR **must** be divided by 1,000,000 to get the price in XTR.
   
  In `snippets/price.liquid` (or whatever your theme uses to display price) replace the content with:
 ```html
  {% assign formatted_price = price | times: rate | divided_by: 1000000 %}
  <img class="small-price-gem" 
       src="https://cdn.shopify.com/s/files/1/0337/0922/8076/t/3/assets/tariGem.svg?v=1584232388" 
       alt="XTR"
  />
  <small aria-hidden="true" class="lower-case">{{ formatted_price }}</small>
  <span class="visually-hidden">{{ price | money }}</span>
  ```            
  
Repeat this process in other files where price is displayed, including `templates/cart.liquid`. The HTML looks 
something like this, but YMMV: 
```html
{% assign global_rate = shop.metaobjects["tari_price"]["tari-price-global"].usd %}
{% for item in cart.items %}
   {% if item.variant.metafields["custom"]["tari_price"] != nil %}  
     {%  assign tari_price = item.variant.metafields["custom"]["tari_price"]  %}
   {% else %}
     {%  assign tari_price = item.variant.price | times: global_rate  %}
   {%  endif %}

  <!-- Wherever the Tari price is displayed -->
  <span class="cart-price">
    <img class="small-price-gem" 
       src="https://cdn.shopify.com/s/files/1/0337/0922/8076/t/3/assets/tariGem.svg?v=1584232388" 
       alt="XTR"
    />
   {{ tari_price | divided_by: 1000000 }}
  </span>

  <!-- Rest of template -->
{% endfor %}
```

* Save the changes to all the files you've edited.
* You should now be able to set the product inventory to the base currency cost, but see prices displayed in Tari on your store.

[metafield]: https://shopify.dev/docs/api/functions/reference/fulfillment-constraints/graphql/common-objects/metafield "Shopify product metafields"
[variant]: https://shopify.dev/docs/api/liquid/objects#variant "Shopify product variants"

### Configure webhooks to interact with your server.

#### Using the CLI utility (recommended)
You can use the accompanying CLI utility to install the required webhooks into your store. This utility will set up 
and configure the webhooks for you. The CLI utility makes use of your Admin API key 
(See [`shopify` command configuration](#configuring-taritools)). You must also have assigned the correct permissions 
as described in the [Enable custom App development](#enable-custom-app-development) section. 

1. Run `taritools shopify webhooks install` to install the required webhooks into your store.
2. Run `taritools shopify webhooks list` to verify that the webhooks have been installed.

#### Manually configuring webhooks
If you don't want to use the CLI utility, or you want to update a webhook configuration, or you broadly want to know 
what the CLI utility is doing under the hood, you can manually configure the webhooks in your store.

1. In your app store admin panel, go to `Settings` -> `Notifications`.
2. Click on the "Webhooks" button.
3. On the **Webhooks** page, take not of the signing secret. This is indicated by the text `Your webhooks will be 
   signed with xxxxxxxxxxxxxxxxxxxxx`. This secret must be assigned to the `TPG_SHOPIFY_HMAC_SECRET` environment 
   variable in your server's `.env` file.
4. Click on the "Create webhook" button.
5. Configure the following webhooks. For each webhook, the format is `JSON`, and the API version is `2024-04`:

| Event          | URL                                                           |
|----------------|---------------------------------------------------------------|
| Order creation | `https://your-server-url.com/shopify/webhook/checkout-create` |
| Product create | `https://your-server-url.com/shopify/webhook/product-create`  |
| Product update | `https://your-server-url.com/shopify/webhook/product-update`  |

# Tari tools

## Configuring `taritools`
The `taritools` CLI utility is configured using the same `.env` file as the Tari Payment Server.

### Shopify commands
The `shopify` command makes use of the following environment variables to work correctly. If these are set in the `.
env` file, they will be used by the CLI utility. If not, you must set them up manually in your environment.

* `TPG_SHOPIFY_SHOP`
* `TPG_SHOPIFY_API_VERSION`
* `TPG_SHOPIFY_ADMIN_ACCESS_TOKEN`
* `TPG_SHOPIFY_API_SECRET`
