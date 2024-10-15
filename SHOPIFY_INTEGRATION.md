# How to set up Shopify Storefront integration with Tari Payment Server

## Initial setup

1. You need to have an existing Shopify store. If you don't have one, you can create one [here](https://www.shopify.com/).
2. Set up your store with the products you want to sell, configure the theme and so on.
3. If you only want to accept Tari payments, you can remove all other payment methods from your store. 
   To do this, go to `Apps and Sales Channels` and remove all pre-installed apps, except for "Online Store".
4. Configure your store to process **Manual Payments** ([Shopify Help](https://help.shopify.com/en/manual/payments/additional-payment-methods/activate-payment-methods)). 
   * To do this, go to `Settings` -> `Payments`. Under `Payment Capture Methods`, select `Manually`.
   * Under `Manual Payment Methods`, select `Create custom payment method`.
   * Enter the name of the payment method as `Tari Payment Server`.
     * Click `Activate`.

## Enable custom App development

1. From your store's admin, go to `Settings` -> `Apps and sales channels`.
2. Click on `Develop apps`.
3. Click on `Create app`.
4. Fill in the details of your app. Give the app the title `Tari integration`, or whatever suits your fancy.
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
   ![Shopify API keys](./assets/shopify_api_keys.png)
9. Click `Install app` and follow the prompts to install the app in your store.
   * Save your Admin API token. **Warning**: You only get to see this once!
   * Save your Storefront API token.
   * ![Shopify API keys](./assets/shopify_credentials.png)

## Configure the store to display prices in Tari.

Shopify makes it difficult to display prices in a currency other than the store's base currency or a 
'recognised' local currency. And in 2024, Shopify has been deprecating APIs that allows apps to alter the 
checkout experience, making it nearly impossible to provide a completely seamless experience for merchants wanting 
to provide an end-to-end Tari payment experience.

Tari Payment Server assumes that your store is using USD as a base currency, and uses the 
configured XTR-USD exchange rate to set a [metafield] on every product [variant] in the store every time the exchange 
rate is updated, as well as a global rate set in a metaobject variable in the odd instance where a product or 
variant price has not been set yet. 

### Important note!

The source of truth for the exchange rate is the value in the Tari Payment Server database, 
_not the store_. If you update the exchange rate in the store, or alter the Tari price in the metafield for a 
product, then any orders using those products will display the wrong price, and you will get a mismatch between 
the amount the payment server is expecting and the final order price on the customer's invoice. 

You can use the [`taritools`]  CLI utility to update the exchange rate; this will reset all the [metafield] values in 
 the shopify product list. 
 
Once your [webhooks](#configure-webhooks-to-interact-with-your-server) are correctly configured,
you can freely update the USD price of products in the store. 
The payment server will detect this update and update the [metafield] value for the product automatically.

### Another important note

The base unit for Tari is  _microTari (μT)_. The `tari_price` metafield always represents the price in μT, 
and therefore any representation of the price in XTR **must** be divided by 1,000,000 to get the price in XTR.
                                   

### Updating the shopify templates

#### Theme template code

The code for your theme teplates is accessible from the Shopify admin console. Go to `Sales channels` | `Online Store`
| `Themes` | `Actions` | `Edit code`.

![Shopify theme editor](./assets/edit_theme.png)

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

## Configure the .env file

The Tari Payment Server uses environment variables to configure the server. You can set these in a `.env` file in the
root of the same directory as the server binary. We strongly recommend using a vault or secret manager to store the
secret keys and other sensitive information and set the environment variables in the shell to avoid having secrets
stored on disk.

TPS assumes that the following environment variables are set when making use of the Shopify API:

- `TPG_SHOPIFY_SHOP`: Your Shopify shop name, e.g. `my-shop.myshopify.com`
- `TPG_SHOPIFY_API_VERSION`: Optional. The API version to use. Default is `2024-04`.
- `TPG_SHOPIFY_STOREFRONT_ACCESS_TOKEN`: 
- `TPG_SHOPIFY_ADMIN_ACCESS_TOKEN`: Your Shopify admin access token. e.g. `shpat_xxxxxxxx`
- `TPG_SHOPIFY_API_SECRET`: Your Shopify API secret. e.g. `aaaaaaaaaaaa`. This is the token you make a note of in 
  [the setup step above](#enable-custom-app-development).
- `TPG_SHOPIFY_API_KEY`: This variable is used to set the API key for your Shopify app. If not set, an error message will be logged, and the value will be set to an empty string.
- `TPG_SHOPIFY_HMAC_CHECKS`. A flag to indicate whether to check the HMAC signature of incoming requests from Shopify.
  Set to `1` to enable HMAC checks, and `0` to disable them. You **almost certainly want to enable this in production.**
- `TPG_SHOPIFY_ORDER_ID_FIELD`. Specify which field should be used as the order id. Must be one of `name` or `id`. 
   Default is `id`.
  
## Configure webhooks to interact with your server.

### Using the CLI utility (recommended)
You can use the accompanying CLI utility to install the required webhooks into your store. This utility will set up 
and configure the webhooks for you. The CLI utility makes use of your Admin API key 
(See [`shopify` command configuration](#configuring-taritools)). You must also have assigned the correct permissions 
as described in the [Enable custom App development](#enable-custom-app-development) section. 

1. Run `taritools shopify webhooks install "https://{my.tari-payment-server.com}"` to install the required webhooks 
   into your store.
2. Run `taritools shopify webhooks list` to verify that the webhooks have been installed.
3. Set the `TPG_SHOPIFY_HMAC_SECRET` environment variable in your server's environment to **the same value** as 
   `TPG_SHOPIFY_API_SECRET`. Shopify uses _different_ secrets for webhooks defined in the Admin UI and the API.

### Manually configuring webhooks
If you don't want to use the CLI utility, or you want to update a webhook configuration, or you broadly want to know 
what the CLI utility is doing under the hood, you can manually configure the webhooks in your store.

1. In your app store admin panel, go to `Settings` -> `Notifications`.
2. Click on the "Webhooks" button.
3. On the **Webhooks** page, take not of the signing secret. This is indicated by the text `Your webhooks will be 
   signed with xxxxxxxxxxxxxxxxxxxxx`. This secret must be assigned to the `TPG_SHOPIFY_HMAC_SECRET` environment 
   variable in your server's `.env` file.
4. Note that Shopify uses _different_ secrets for webhooks defined in the Admin UI and the API.
5. Click on the "Create webhook" button.
6. Configure the following webhooks. For each webhook, the format is `JSON`, and the API version is `2024-04`:

| Event          | URL                                                           |
|----------------|---------------------------------------------------------------|
| Order creation | `https://your-server-url.com/shopify/webhook/checkout-create` |
| Product create | `https://your-server-url.com/shopify/webhook/product-create`  |
| Product update | `https://your-server-url.com/shopify/webhook/product-update`  |

## Editing customer notifications

You can edit any of the Customer Notification templates to improve the user experience of your store, but at the 
bare minimum, we suggest updating the `Order Confirmation Email` template to reflect the Tari prices in the order 
and most importantly, communicating the order ID to the customer.

The template is edited in the Admin console under `Settings` -> `Notifications` -> `Order Confirmation`.
(https://admin.shopify.com/store/{store_id}/email_templates/order_confirmation)

You can reflect prices in Tari using the following template:

```liquid
{% assign tari_subtotal_price = 0 %}              
<table class="row">
  {% for line in subtotal_line_items %}
    <!-- The tari price is automatically set by Tari Payment Server if webhooks have been correctly configured -->
    {% assign tari_price = line.variant.metafields["custom"]["tari_price"] %}
    {% assign line_total = tari_price | times: line.quantity %}
    {% assign tari_subtotal_price = tari_subtotal_price | plus: line_total %}
    <!--- snip --->
        <td class="subtotal-line__value">
          <img class="price-gem" src="https://cdn.shopify.com/s/files/1/0337/0922/8076/files/TariGem.png?v=1584467930" alt="Tari" height="13"/>
          <strong>{{ tari_subtotal_price | divided_by: 1000000 }}</strong>
        </td>
    <!--- snip --->
    {% endfor %}
    
    <tr class="subtotal-line">
      <td class="subtotal-line__title"><span>Subtotal</span></td>
      <td class="subtotal-line__value">
        <img class="price-gem" src="https://cdn.shopify.com/s/files/1/0337/0922/8076/files/TariGem.png?v=1584467930" height="13"/>
        <strong>{{ tari_subtotal_price | divided_by: 1000000 }}</strong>
      </td>
    </tr>
    
    <!-- For taxes and shipping, there's no way to get the global rate, so calculate it manually -->
    {% assign average_rate = tari_subtotal_price | divided_by: subtotal_price %} 
    {% assign tari_shipping_price = shipping_price | times: average_rate %} 
    
    <tr class="subtotal-line">
      <td class="subtotal-line__title"> <span>Shipping</span></td>
      <td class="subtotal-line__value">
        <img class="price-gem" src="https://cdn.shopify.com/s/files/1/0337/0922/8076/files/TariGem.png?v=1584467930" height="13"/>
        <strong>{{ tari_shipping_price | divided_by: 1000000 }}</strong>
      </td>
    </tr>
    
    <!-- repeat this pattern for duties, shipping and discounts -->
</table>
{% assign tari_total_price = tari_subtotal_price | plus: tari_shipping_price | plus: tari_taxes | plus: tari_duties %}
<table class="row subtotal-table subtotal-table--total">
  <tr class="subtotal-line">
    <td class="subtotal-line__title"><span>Total due</span></td>
    <td class="subtotal-line__value">
      <img class="price-gem" src="https://cdn.shopify.com/s/files/1/0337/0922/8076/files/TariGem.png?v=1584467930" alt="Tari" height="20"/>
      <strong>{{ tari_total_price | divided_by: 1000000 }}</strong>
    </td>
  </tr>
</table>
```

You can also access some useful metadata about the order in your template. Here is a simple example:

```liquid
<h1>Metadata</h1>
{% assign tps_wallet_address = "0859fb3d6696579310c220d204cb21437d6658d0a05af1c8cd54fffd8725344352" %}
{% capture tari_payment_link %}
  tari://pay?shop={{shop.id}}&order_id={{id}}&amount={{tari_total_price}}&send_to={{ tps_wallet_address }}
{% endcapture %}
<ul>
  <li> Order id: {{ id }} </li> 
  <li> Order status: {{ order_status_url }} </li>
  <li> Aurora link (if viewing this on your mobile phone): {{ tari_payment_link }}</li>
</ul> 
<div>
  <h2>Autogenererated QR code</h2>
  <img src="https://api.qrserver.com/v1/create-qr-code/?size=150x150&data={{ tari_payment_link | url_encode }}"/>
</div> 
```

You'll notice that you need to hard-code the hot wallet address into the template. This is because Shopify does not 
provide access to global metaobject data in the email templates. 

You might also want to replace `qrserver.com` with your own QR code generator.

Shopify also scrubs links from anchor tags in the email templates (e.g. `<a href="tari://...">`), so clickable links 
to trigger deep links will not work. (If there's a workaround, please let us know!)

## Shopify commands in Taritools
The `taritools shopify` command makes use of the following environment variables to work correctly. 
If these are set in the `.env` file, they will be used by the CLI utility. 
If not, you must set them up manually in your environment.

* `TPG_SHOPIFY_SHOP`
* `TPG_SHOPIFY_API_VERSION`
* `TPG_SHOPIFY_ADMIN_ACCESS_TOKEN`
* `TPG_SHOPIFY_API_SECRET`
  
[`taritools`]: ./taritools/README.md "Taritools README"
