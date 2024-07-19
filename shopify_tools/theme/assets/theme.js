/*!
 * enquire.js v2.1.2 - Awesome Media Queries in JavaScript
 * Copyright (c) 2014 Nick Williams - http://wicky.nillia.ms/enquire.js
 * License: MIT (http://www.opensource.org/licenses/mit-license.php)
 */
!function(a,b,c){var d=window.matchMedia;"undefined"!=typeof module&&module.exports?module.exports=c(d):"function"==typeof define&&define.amd?define(function(){return b[a]=c(d)}):b[a]=c(d)}("enquire",this,function(a){"use strict";function b(a,b){var c,d=0,e=a.length;for(d;e>d&&(c=b(a[d],d),c!==!1);d++);}function c(a){return"[object Array]"===Object.prototype.toString.apply(a)}function d(a){return"function"==typeof a}function e(a){this.options=a,!a.deferSetup&&this.setup()}function f(b,c){this.query=b,this.isUnconditional=c,this.handlers=[],this.mql=a(b);var d=this;this.listener=function(a){d.mql=a,d.assess()},this.mql.addListener(this.listener)}function g(){if(!a)throw new Error("matchMedia not present, legacy browsers require a polyfill");this.queries={},this.browserIsIncapable=!a("only all").matches}return e.prototype={setup:function(){this.options.setup&&this.options.setup(),this.initialised=!0},on:function(){!this.initialised&&this.setup(),this.options.match&&this.options.match()},off:function(){this.options.unmatch&&this.options.unmatch()},destroy:function(){this.options.destroy?this.options.destroy():this.off()},equals:function(a){return this.options===a||this.options.match===a}},f.prototype={addHandler:function(a){var b=new e(a);this.handlers.push(b),this.matches()&&b.on()},removeHandler:function(a){var c=this.handlers;b(c,function(b,d){return b.equals(a)?(b.destroy(),!c.splice(d,1)):void 0})},matches:function(){return this.mql.matches||this.isUnconditional},clear:function(){b(this.handlers,function(a){a.destroy()}),this.mql.removeListener(this.listener),this.handlers.length=0},assess:function(){var a=this.matches()?"on":"off";b(this.handlers,function(b){b[a]()})}},g.prototype={register:function(a,e,g){var h=this.queries,i=g&&this.browserIsIncapable;return h[a]||(h[a]=new f(a,i)),d(e)&&(e={match:e}),c(e)||(e=[e]),b(e,function(b){d(b)&&(b={match:b}),h[a].addHandler(b)}),this},unregister:function(a,b){var c=this.queries[a];return c&&(b?c.removeHandler(b):(c.clear(),delete this.queries[a])),this}},new g});

/* Simple jQuery Equal Heights @version 1.5.1. Copyright (c) 2013 Matt Banks. Dual licensed under the MIT and GPL licenses. */
!function(a){a.fn.equalHeights=function(){var b=0,c=a(this);return c.each(function(){var c=a(this).innerHeight();c>b&&(b=c)}),c.css("height",b)},a("[data-equal]").each(function(){var b=a(this),c=b.data("equal");b.find(c).equalHeights()})}(jQuery);

/* Run function after window resize */
var afterResize=(function(){var t={};return function(callback,ms,uniqueId){if(!uniqueId){uniqueId="Don't call this twice without a uniqueId";}if(t[uniqueId]){clearTimeout(t[uniqueId]);}t[uniqueId]=setTimeout(callback,ms);};})();

//* Ajaxify.js.liquid *//

/*============================================================================
  (c) Copyright 2015 Shopify Inc. Author: Carson Shold (@cshold). All Rights Reserved.

  Plugin Documentation - https://shopify.github.io/Timber/#ajax-cart

  Ajaxify the add to cart experience and flip the button for inline confirmation,
  show the cart in a modal, or a 3D drawer.

  This file includes:
    - Basic Shopify Ajax API calls
    - Ajaxify plugin

  This requires:
    - jQuery 1.8+
    - handlebars.min.js (for cart template)
    - modernizer.min.js
    - snippet/ajax-cart-template.liquid

  JQUERY API (c) Copyright 2009-2015 Shopify Inc. Author: Caroline Schnapp. All Rights Reserved.
  Includes slight modifications to addItemFromForm.
==============================================================================*/
if ((typeof Shopify) === 'undefined') { Shopify = {}; }

/*============================================================================
  API Helper Functions
==============================================================================*/
function attributeToString(attribute) {
  if ((typeof attribute) !== 'string') {
    attribute += '';
    if (attribute === 'undefined') {
      attribute = '';
    }
  }
  return jQuery.trim(attribute);
}

/*============================================================================
  API Functions
  - Shopify.format money is defined in option_selection.js.
    If that file is not included, it is redefined here.
==============================================================================*/
if ( !Shopify.formatMoney ) {
  Shopify.formatMoney = function(cents, format) {
    var value = '',
        placeholderRegex = /\{\{\s*(\w+)\s*\}\}/,
        formatString = (format || this.money_format);

    if (typeof cents == 'string') {
      cents = cents.replace('.','');
    }

    function defaultOption(opt, def) {
      return (typeof opt == 'undefined' ? def : opt);
    }

    function formatWithDelimiters(number, precision, thousands, decimal) {
      precision = defaultOption(precision, 2);
      thousands = defaultOption(thousands, ',');
      decimal   = defaultOption(decimal, '.');

      if (isNaN(number) || number == null) {
        return 0;
      }

      number = (number/100.0).toFixed(precision);

      var parts   = number.split('.'),
          dollars = parts[0].replace(/(\d)(?=(\d\d\d)+(?!\d))/g, '$1' + thousands),
          cents   = parts[1] ? (decimal + parts[1]) : '';

      return dollars + cents;
    }

    switch(formatString.match(placeholderRegex)[1]) {
      case 'amount':
        value = formatWithDelimiters(cents, 2);
        break;
      case 'amount_no_decimals':
        value = formatWithDelimiters(cents, 0);
        break;
      case 'amount_with_comma_separator':
        value = formatWithDelimiters(cents, 2, '.', ',');
        break;
      case 'amount_no_decimals_with_comma_separator':
        value = formatWithDelimiters(cents, 0, '.', ',');
        break;
    }

    return formatString.replace(placeholderRegex, value);
  };
}

Shopify.onProduct = function(product) {
  // alert('Received everything we ever wanted to know about ' + product.title);
};

Shopify.onCartUpdate = function(cart) {
  // alert('There are now ' + cart.item_count + ' items in the cart.');
};

Shopify.updateCartNote = function(note, callback) {
  var params = {
    type: 'POST',
    url: '/cart/update.js',
    data: 'note=' + attributeToString(note),
    dataType: 'json',
    success: function(cart) {
      if ((typeof callback) === 'function') {
        callback(cart);
      }
      else {
        Shopify.onCartUpdate(cart);
      }
    },
    error: function(XMLHttpRequest, textStatus) {
      Shopify.onError(XMLHttpRequest, textStatus);
    }
  };
  jQuery.ajax(params);
};

Shopify.onError = function(XMLHttpRequest, textStatus) {
  var data = eval('(' + XMLHttpRequest.responseText + ')');
  if (!!data.message) {
    alert(data.message + '(' + data.status  + '): ' + data.description);
  } else {
    alert('Error : ' + Shopify.fullMessagesFromErrors(data).join('; ') + '.');
  }
};

/*============================================================================
  POST to cart/add.js returns the JSON of the line item associated with the added item
==============================================================================*/
Shopify.addItem = function(variant_id, quantity, callback) {
  var quantity = quantity || 1;
  var params = {
    type: 'POST',
    url: '/cart/add.js',
    data: 'quantity=' + quantity + '&id=' + variant_id,
    dataType: 'json',
    success: function(line_item) {
      if ((typeof callback) === 'function') {
        callback(line_item);
      }
      else {
        Shopify.onItemAdded(line_item);
      }
    },
    error: function(XMLHttpRequest, textStatus) {
      Shopify.onError(XMLHttpRequest, textStatus);
    }
  };
  jQuery.ajax(params);
};

/*============================================================================
  POST to cart/add.js returns the JSON of the line item
    - Allow use of form element instead of id
    - Allow custom error callback
==============================================================================*/
Shopify.addItemFromForm = function(form, callback, errorCallback) {
  var params = {
    type: 'POST',
    url: '/cart/add.js',
    data: jQuery(form).serialize(),
    dataType: 'json',
    success: function(line_item) {
      if ((typeof callback) === 'function') {
        callback(line_item, form);
      }
      else {
        Shopify.onItemAdded(line_item, form);
      }
    },
    error: function(XMLHttpRequest, textStatus) {
      if ((typeof errorCallback) === 'function') {
        errorCallback(XMLHttpRequest, textStatus);
      }
      else {
        Shopify.onError(XMLHttpRequest, textStatus);
      }
    }
  };
  jQuery.ajax(params);
};

// Get from cart.js returns the cart in JSON
Shopify.getCart = function(callback) {
  jQuery.getJSON('/cart.js', function (cart, textStatus) {
    if ((typeof callback) === 'function') {
      callback(cart);
    }
    else {
      Shopify.onCartUpdate(cart);
    }
  });
};

// GET products/<product-handle>.js returns the product in JSON
Shopify.getProduct = function(handle, callback) {
  jQuery.getJSON('/products/' + handle + '.js', function (product, textStatus) {
    if ((typeof callback) === 'function') {
      callback(product);
    }
    else {
      Shopify.onProduct(product);
    }
  });
};

// POST to cart/change.js returns the cart in JSON
Shopify.changeItem = function(line, quantity, callback) {
  var params = {
    type: 'POST',
    url: '/cart/change.js',
    data:  'quantity=' + quantity + '&line=' + line,
    dataType: 'json',
    success: function(cart) {
      if ((typeof callback) === 'function') {
        callback(cart);
      }
      else {
        Shopify.onCartUpdate(cart);
      }
    },
    error: function(XMLHttpRequest, textStatus) {
      Shopify.onError(XMLHttpRequest, textStatus);
    }
  };
  jQuery.ajax(params);
};

/*============================================================================
  Ajaxify Shopify Add To Cart
==============================================================================*/
var ajaxifyShopify = (function(module, $) {

  'use strict';

  // Public functions
  var init;

  // Private general variables
  var settings, isUpdating, cartInit, $drawerHeight, $cssTransforms, $cssTransforms3d, $w, $body, $html;

  // Private plugin variables
  var $formContainer, $btnClass, $wrapperClass, $addToCart, $flipClose, $flipCart, $flipContainer, $cartCountSelector, $cartCostSelector, $toggleCartButton, $modal, $cartContainer, $drawerCaret, $modalContainer, $modalOverlay, $closeCart, $drawerContainer, $prependDrawerTo, $callbackData={};

  // Private functions
  var updateCountPrice, flipSetup, revertFlipButton, modalSetup, showModal, sizeModal, hideModal, drawerSetup, showDrawer, hideDrawer, sizeDrawer, loadCartImages, formOverride, itemAddedCallback, itemErrorCallback, cartUpdateCallback, setToggleButtons, flipCartUpdateCallback, buildCart, cartTemplate, adjustCart, adjustCartCallback, createQtySelectors, qtySelectors, scrollTop, toggleCallback, validateQty;

  /*============================================================================
    Initialise the plugin and define global options
  ==============================================================================*/
  init = function (options) {

    // Default settings
    settings = {
      method: 'drawer', // Method options are drawer, modal, and flip. Default is drawer.
      formSelector: '[data-product-form]',
      cartFormSelector: '[data-cart-form]',
      addToCartSelector: 'input[type="submit"]',
      cartCountSelector: null,
      cartCostSelector: null,
      toggleCartButton: null,
      btnClass: null,
      wrapperClass: null,
      useCartTemplate: false,
      moneyFormat: '${{amount}}',
      disableAjaxCart: false,
      enableQtySelectors: true,
      prependDrawerTo: 'body',
      onToggleCallback: null
    };

    // Override defaults with arguments
    $.extend(settings, options);

    // Make sure method is lower case
    settings.method = settings.method.toLowerCase();

    // Select DOM elements
    $formContainer     = $(settings.formSelector);
    $btnClass          = settings.btnClass;
    $wrapperClass      = settings.wrapperClass;
    $addToCart         = $formContainer.find(settings.addToCartSelector);
    $flipContainer     = null;
    $flipClose         = null;
    $cartCountSelector = $(settings.cartCountSelector);
    $cartCostSelector  = $(settings.cartCostSelector);
    $toggleCartButton  = $(settings.toggleCartButton);
    $modal             = null;
    $prependDrawerTo   = $(settings.prependDrawerTo);

    // CSS Checks
    $cssTransforms   = Modernizr.csstransforms;
    $cssTransforms3d = Modernizr.csstransforms3d;

    // General Selectors
    $w    = $(window);
    $body = $('body');
    $html = $('html');

    // Track cart activity status
    isUpdating = false;

    // Setup ajax quantity selectors on the any template if enableQtySelectors is true
    if (settings.enableQtySelectors) {
      qtySelectors();
    }

    // Enable the ajax cart
    if (!settings.disableAjaxCart) {
      // Handle each case add to cart method
      switch (settings.method) {
        case 'flip':
          flipSetup();
          break;

        case 'modal':
          if (!$('#ajaxifyCart-overlay').length) {
            modalSetup();
          }
          break;

        case 'drawer':
          drawerSetup();
          break;
      }

      // Escape key closes cart
      $(document).keyup( function (evt) {
        if (evt.keyCode == 27) {
          switch (settings.method) {
            case 'flip':
            case 'drawer':
              hideDrawer();
              break;
            case 'modal':
              hideModal();
              break;
          }
        }
      });

      if ( $addToCart.length ) {
        // Take over the add to cart form submit
        formOverride();
      }
    }

    // Run this function in case we're using the quantity selector outside of the cart
    adjustCart();
  };

  updateCountPrice = function (cart) {
    if ($cartCountSelector) {
      $cartCountSelector.html(cart.item_count).removeClass('hidden-count');

      if (cart.item_count === 0) {
        $cartCountSelector.addClass('hidden-count');
      }
    }
    if ($cartCostSelector) {
      $cartCostSelector.html(Shopify.formatMoney(cart.total_price, settings.moneyFormat));
    }
  };

  flipSetup = function () {
    // Build and append the drawer in the DOM
    drawerSetup();

    // Stop if there is no add to cart button
    if ( !$addToCart.length ) {
      return
    }

    // Wrap the add to cart button in a div
    $addToCart.addClass('flip-front').wrap('<div class="flip"></div>');

    var cartUrl = $cartContainer.data('cart-url');
    // Write a (hidden) Checkout button, a loader, and the extra view cart button
    var checkoutBtn = $('<a href="' + cartUrl + '" class="flip-back" style="background-color: #C00; color: #fff;" id="flip-checkout">' + "Checkout" + '</a>').addClass($btnClass),
        flipLoader = $('<span class="ajaxifyCart-loader"></span>'),
        flipExtra = $('<div class="flip-extra">' + "or" + ' <a href="#" class="flip-cart">' + "View Cart" + ' (<span></span>)</a></div>');

    // Append checkout button and loader
    checkoutBtn.insertAfter($addToCart);
    flipLoader.insertAfter(checkoutBtn);

    // Setup new selectors
    $flipContainer = $('.flip');

    if (!$cssTransforms3d) {
      $flipContainer.addClass('no-transforms')
    }

    // Setup extra selectors once appended
    flipExtra.insertAfter($flipContainer);
    $flipCart = $('.flip-cart');

    $flipCart.on('click', function(e) {
      e.preventDefault();
      showDrawer(true);
    });

    // Reset the button if a user changes a variation
    $('input[type="checkbox"], input[type="radio"], select', $formContainer).on('click', function() {
      revertFlipButton();
    })
  };

  revertFlipButton = function () {
    $flipContainer.removeClass('is-flipped');
  };

  modalSetup = function () {
    // Create modal DOM elements with handlebars.js template
    var source   = $("#modalTemplate").html(),
        template = Handlebars.compile(source);

    // Append modal and overlay to body
    $body.append(template).append('<div id="ajaxifyCart-overlay"></div>');

    // Modal selectors
    $modalContainer = $('#ajaxifyModal');
    $modalOverlay   = $('#ajaxifyCart-overlay');
    $cartContainer  = $('#ajaxifyCart');

    // Close modal when clicking the overlay
    $modalOverlay.on('click', hideModal);

    // Create a close modal button
    $modalContainer.prepend('<button class="ajaxifyCart--close" title="' + "Close Cart" + '">' + "Close Cart" + '</button>');
    $closeCart = $('.ajaxifyCart--close');
    $closeCart.on('click', hideModal);

    // Add a class if CSS translate isn't available
    if (!$cssTransforms) {
      $modalContainer.addClass('no-transforms')
    }

    // Update modal position on screen changes
    $(window).on({
      orientationchange: function(e) {
        if ($modalContainer.hasClass('is-visible')) {
          sizeModal('resize');
        }
      }, resize: function(e) {
        // IE8 fires this when overflow on body is changed. Ignore IE8.
        if ($modalContainer.hasClass('is-visible')) {
          sizeModal('resize');
        }
      }
    });

    // Toggle modal with cart button
    setToggleButtons();
  };

  showModal = function (toggle) {
    $body.addClass('ajaxify-modal--visible');
    // Build the cart if it isn't already there
    if ( !cartInit && toggle ) {
      Shopify.getCart(cartUpdateCallback);
    } else {
      sizeModal();
    }
  };

  sizeModal = function(isResizing) {
    if (!isResizing) {
      $modalContainer.css('opacity', 0);
    }

    // Position modal by negative margin
    $modalContainer.css({
      'margin-left': - ($modalContainer.outerWidth() / 2),
      'opacity': 1
    });

    $modalContainer.addClass('is-visible');

    scrollTop();

    toggleCallback({
      'is_visible': true
    });
  };

  hideModal = function (e) {
    $body.removeClass('ajaxify-modal--visible');
    if (e) {
      e.preventDefault();
    }

    if ($modalContainer) {
      $modalContainer.removeClass('is-visible');
      $body.removeClass('ajaxify-lock');
    }

    toggleCallback({
      'is_visible': false
    });
  };

  drawerSetup = function () {
    // Create drawer DOM elements with handlebars.js template
    var source   = $("#drawerTemplate").html(),
        template = Handlebars.compile(source),
        data = {
          wrapperClass: $wrapperClass
        };

    // Append drawer (defaults to body) if it doesn't already exist
    if (!$('#ajaxifyDrawer').length) {
      $prependDrawerTo.prepend(template(data));
    }

    // Drawer selectors
    $drawerContainer = $('#ajaxifyDrawer');
    $cartContainer   = $('#ajaxifyCart');
    $drawerCaret     = $('.ajaxifyDrawer-caret > span');

    // Toggle drawer with cart button
    setToggleButtons();

    // Position caret and size drawer on resize if drawer is visible
    var timeout;
    $(window).resize(function() {
      clearTimeout(timeout);
      timeout = setTimeout(function(){
        if ($drawerContainer.hasClass('is-visible')) {
          positionCaret();
          sizeDrawer();
        }
      }, 500);
    });

    // Position the caret the first time
    positionCaret();

    // Position the caret
    function positionCaret() {
      if ($toggleCartButton.offset()) {
        // Get the position of the toggle button to align the carat with
        var togglePos = $toggleCartButton.offset(),
            toggleWidth = $toggleCartButton.outerWidth(),
            toggleMiddle = togglePos.left + toggleWidth/2;

        $drawerCaret.css('left', toggleMiddle + 'px');
      }
    }
  };

  showDrawer = function (toggle) {
    // If we're toggling with the flip method, use a special callback
    if (settings.method == 'flip') {
      Shopify.getCart(flipCartUpdateCallback);
    }
    // opening the drawer for the first time
    else if ( !cartInit && toggle) {
      Shopify.getCart(cartUpdateCallback);
    }
    // simple toggle? just size it
    else if ( cartInit && toggle ) {
      sizeDrawer();
    }

    // Show the drawer
    $drawerContainer.addClass('is-visible');

    scrollTop();

    toggleCallback({
      'is_visible': true
    });
  };

  hideDrawer = function () {
    $drawerContainer.removeAttr('style').removeClass('is-visible');
    scrollTop();
    toggleCallback({
      'is_visible': false
    });
  };

  sizeDrawer = function ($empty) {
    if ($empty) {
      $drawerContainer.css('height', '0px');
    } else {
      $drawerHeight = $cartContainer.outerHeight();
      $('.cart-row img').css('width', 'auto'); // fix Chrome image size bug
      $drawerContainer.css('height',  $drawerHeight + 'px');
    }
  };

  loadCartImages = function () {
    // Size cart once all images are loaded
    var cartImages = $('img', $cartContainer),
        count = cartImages.length,
        index = 0;

    cartImages.on('load', function() {
      index++;

      if (index==count) {
        switch (settings.method) {
          case 'modal':
            sizeModal();
            break;
          case 'flip':
          case 'drawer':
            sizeDrawer();
            break;
        }
      }
    });
  };

  formOverride = function () {
    $formContainer.submit(function(e) {
      e.preventDefault();

      // Add class to be styled if desired
      $addToCart.removeClass('is-added').addClass('is-adding');

      // Remove any previous quantity errors
      $('.qty-error').remove();

      Shopify.addItemFromForm(e.target, itemAddedCallback, itemErrorCallback);

      // Set the flip button to a loading state
      switch (settings.method) {
        case 'flip':
          $flipContainer.addClass('flip--is-loading');
          break;
      }
    });
  };

  itemAddedCallback = function (product) {
    $addToCart.removeClass('is-adding').addClass('is-added');

    // Slight delay of flip to mimic a longer load
    switch (settings.method) {
      case 'flip':
        setTimeout(function () {
          $flipContainer.removeClass('flip--is-loading').addClass('is-flipped');
        }, 600);
        break;
    }
    Shopify.getCart(cartUpdateCallback);
  };

  itemErrorCallback = function (XMLHttpRequest, textStatus) {
    switch (settings.method) {
      case 'flip':
        $flipContainer.removeClass('flip--is-loading');
        break;
    }

    var data = eval('(' + XMLHttpRequest.responseText + ')');
    if (!!data.message) {
      if (data.status == 422) {
        $formContainer.after('<div class="errors qty-error">'+ data.description +'</div>')
      }
    }
  };

  cartUpdateCallback = function (cart) {
    // Update quantity and price
    updateCountPrice(cart);

    switch (settings.method) {
      case 'flip':
        $('.flip-cart span').html(cart.item_count);
        break;
      case 'modal':
        buildCart(cart);
        break;
      case 'drawer':
        buildCart(cart);
        if ( !$drawerContainer.hasClass('is-visible') ) {
          showDrawer();
        } else {
          scrollTop();
        }
        break;
    }
  };

  setToggleButtons = function () {
    // Reselect the element in case it just loaded
    $toggleCartButton  = $(settings.toggleCartButton);

    if ($toggleCartButton) {
      // Turn it off by default, in case it's initialized twice
      $toggleCartButton.off('click');

      // Toggle the cart, based on the method
      $toggleCartButton.on('click', function(e) {
        e.preventDefault();

        switch (settings.method) {
          case 'modal':
            if ( $modalContainer.hasClass('is-visible') ) {
              hideModal();
            } else {
              showModal(true);
            }
            break;
          case 'drawer':
          case 'flip':
            if ( $drawerContainer.hasClass('is-visible') ) {
              hideDrawer();
            } else {
              showDrawer(true);
            }
            break;
        }

      });

    }
  };

  flipCartUpdateCallback = function (cart) {
    buildCart(cart);
  };

  buildCart = function (cart) {
    // Empty cart if using default layout or not using the .load method to get /cart
    if (!settings.useCartTemplate || cart.item_count === 0) {
      $cartContainer.empty();
    }

    // Show empty cart
    if (cart.item_count === 0) {
      $cartContainer.append('<h2>' + "Your cart is currently empty." + '</h2><span class="cart--continue-message">' + "Continue browsing \u003ca href=\"\"\u003ehere\u003c\/a\u003e." + '</span><span class="cart--cookie-message">' + "Enable cookies to use the shopping cart" + '</span>');

      switch (settings.method) {
        case 'modal':
          sizeModal('resize');
          break;
        case 'flip':
        case 'drawer':
          sizeDrawer();

          if (!$drawerContainer.hasClass('is-visible') && cartInit) {
            sizeDrawer(true);
          }
          break;
      }
      return;
    }

    // Use the /cart template, or Handlebars.js layout based on theme settings
    if (settings.useCartTemplate) {
      cartTemplate(cart);
      return;
    }

    // Handlebars.js cart layout
    var items = [],
        item = {},
        data = {};

    var source   = $("#cartTemplate").html(),
        template = Handlebars.compile(source);

    // Add each item to our handlebars.js data
    $.each(cart.items, function(index, cartItem) {

      var itemAdd = cartItem.quantity + 1,
          itemMinus = cartItem.quantity - 1,
          itemQty = cartItem.quantity + ' x';

      /* Hack to get product image thumbnail
       *   - Remove file extension, add _small, and re-add extension
       *   - Create server relative link
      */
      var prodImg = cartItem.image.replace(/(\.[^.]*)$/, "_small$1").replace('http:', '');
      var prodName = cartItem.title.replace(/(\-[^-]*)$/, "");
      var prodVariation = cartItem.title.replace(/^[^\-]*/, "").replace(/-/, "");

      // Create item's data object and add to 'items' array
      item = {
        key: cartItem.key,
        line: index + 1, // Shopify uses a 1+ index in the API
        url: cartItem.url,
        img: prodImg,
        name: prodName,
        variation: prodVariation,
        itemAdd: itemAdd,
        itemMinus: itemMinus,
        itemQty: itemQty,
        price: Shopify.formatMoney(cartItem.price, settings.moneyFormat)
      };

      items.push(item);
    });

    // Gather all cart data and add to DOM
    data = {
      items: items,
      totalPrice: Shopify.formatMoney(cart.total_price, settings.moneyFormat),
      btnClass: $btnClass
    }
    $cartContainer.append(template(data));

    // With new elements we need to relink the adjust cart functions
    adjustCart();

    // Setup close modal button and size drawer
    switch (settings.method) {
      case 'modal':
        loadCartImages();
        break;
      case 'flip':
      case 'drawer':
        if (cart.item_count > 0) {
          loadCartImages();
        } else {
          sizeDrawer(true);
        }
        break;
      default:
        break;
    }

    // Mark the cart as built
    cartInit = true;
  };

  cartTemplate = function (cart) {
    var cartUrl = $cartContainer.data('cart-url');

    $cartContainer.load(cartUrl + ' ' + settings.cartFormSelector, function() {

      // With new elements we need to relink the adjust cart functions
      adjustCart();

      // Size drawer at this point
      switch (settings.method) {
        case 'modal':
          loadCartImages();
          break;
        case 'flip':
        case 'drawer':
          if (cart.item_count > 0) {
            loadCartImages();
          } else {
            sizeDrawer(true);
          }
          break;
        default:
          break;
      }

      // Mark the cart as built
      cartInit = true;
    });
  }

  adjustCart = function () {
    // This function runs on load, and when the cart is reprinted

    // Create ajax friendly quantity fields and remove links in the ajax cart
    if (settings.useCartTemplate) {
      createQtySelectors();
    }

    // Prevent cart from being submitted while quantities are changing
    $body.on('submit', 'form.cart-form', function(evt) {
      if (isUpdating) {
        evt.preventDefault();
      }
    });

    // Update quantify selectors
    var qtyAdjust = $('.ajaxifyCart--qty span');

    // Add or remove from the quantity
    qtyAdjust.off('click');
    qtyAdjust.on('click', function() {
      if (isUpdating) {
        return;
      }

      var el = $(this),
          line = el.data('line'),
          qtySelector = el.siblings('.ajaxifyCart--num'),
          qty = parseInt( qtySelector.val() );

      qty = validateQty(qty);

      // Add or subtract from the current quantity
      if (el.hasClass('ajaxifyCart--add')) {
        qty = qty + 1;
      } else {
        qty = qty <= 0 ? 0 : qty - 1;
      }

      // If it has a data-line, update the cart.
      // Otherwise, just update the input's number
      if (line) {
        updateQuantity(line, qty);
      } else {
        qtySelector.val(qty);
      }

    });

    // Update quantity based on input on change
    var qtyInput = $('.ajaxifyCart--num');
    qtyInput.off('change');
    qtyInput.on('change', function() {
      if (isUpdating) {
        return;
      }

      var el = $(this),
          line = el.data('line'),
          qty = el.val();

      // Make sure we have a valid integer
      if( (parseFloat(qty) == parseInt(qty)) && !isNaN(qty) ) {
        // We have a number!
      } else {
        // Not a number. Default to 1.
        el.val(1);
        return;
      }

      // If it has a data-line, update the cart
      if (line) {
        updateQuantity(line, qty);
      }
    });

    // Highlight the text when focused
    qtyInput.off('focus');
    qtyInput.on('focus', function() {
      var el = $(this);
      setTimeout(function() {
        el.select();
      }, 50);
    });

    // Completely remove product
    $('.ajaxifyCart--remove').on('click', function(e) {
      var el = $(this),
          line = el.data('line') || null,
          qty = 0;

      // Without a data-line, let the default link action take over
      if (!line) {
        return;
      }

      e.preventDefault();
      updateQuantity(line, qty);
    });

    function updateQuantity(line, qty) {
      isUpdating = true;

      // Add activity classes when changing cart quantities
      if (!settings.useCartTemplate) {
        var row = $('.ajaxifyCart--row[data-line="' + line + '"]').addClass('ajaxifyCart--is-loading');
      } else {
        var row = $('.cart-row[data-line="' + line + '"]').addClass('ajaxifyCart--is-loading');
      }

      if ( qty === 0 ) {
        row.addClass('is-removed');
      }

      // Slight delay to make sure removed animation is done
      setTimeout(function() {
        Shopify.changeItem(line, qty, adjustCartCallback);
      }, 250);
    }

    // Save note anytime it's changed
    var noteArea = $('textarea[name="note"]');
    noteArea.off('change');
    noteArea.on('change', function() {
      var newNote = $(this).val();

      // Simply updating the cart note in case they don't click update/checkout
      Shopify.updateCartNote(newNote, function(cart) {});
    });

    if (window.Shopify && Shopify.StorefrontExpressButtons) {
      Shopify.StorefrontExpressButtons.initialize();
    }
  };

  adjustCartCallback = function (cart) {

    // Update quantity and price
    updateCountPrice(cart);

    // Hide the modal or drawer if we're at 0 items
    if ( cart.item_count === 0 ) {
      // Handle each add to cart method
      switch (settings.method) {
        case 'modal':
          break;
        case 'flip':
        case 'drawer':
          hideDrawer();
          break;
      }
    }

    // Reprint cart on short timeout so you don't see the content being removed
    setTimeout(function() {
      isUpdating = false;
      Shopify.getCart(buildCart);
    }, 150)
  };

  createQtySelectors = function() {
    // If there is a normal quantity number field in the ajax cart, replace it with our version
    if ($('input[type="number"]', $cartContainer).length) {
      $('input[type="number"]', $cartContainer).each(function() {
        var el = $(this),
        currentQty = parseInt(el.val());

        var itemAdd = currentQty + 1,
          itemMinus = currentQty - 1,
          itemQty = currentQty + ' x';

        var source = $("#ajaxifyQty").html(),
          template = Handlebars.compile(source),
          data = {
            line: el.attr('data-line'),
            itemQty: itemQty,
            itemAdd: itemAdd,
            itemMinus: itemMinus
          };

        // Append new quantity selector then remove original
        el.after(template(data)).remove();
      });
    }

    var cartChangeURL = $cartContainer.data('cart-change-url');

    // If there is a regular link to remove an item, add attributes needed to ajaxify it
    if ($('a[href^="' + cartChangeURL + '"]', $cartContainer).length) {
      $('a[href^="' + cartChangeURL + '"]', $cartContainer).each(function() {
        var el = $(this).addClass('ajaxifyCart--remove');
      });
    }
  };

  qtySelectors = function() {
    // Change number inputs to JS ones, similar to ajax cart but without API integration.
    // Make sure to add the existing name and id to the new input element
    var numInputs = $('input[type="number"]');

    // Qty selector has a minimum of 1 on the product page
    // and 0 in the cart (determined on qty click)
    var qtyMin = 0;

    if (numInputs.length) {
      numInputs.each(function() {
        var el = $(this),
            currentQty = parseInt(el.val()),
            inputName = el.attr('name'),
            inputId = el.attr('id');

        var itemAdd = currentQty + 1,
            itemMinus = currentQty - 1,
            itemQty = currentQty;

        var source   = $("#jsQty").html(),
            template = Handlebars.compile(source),
            data = {
              key: el.data('id'),
              itemQty: itemQty,
              itemAdd: itemAdd,
              itemMinus: itemMinus,
              inputName: inputName,
              inputId: inputId
            };

        // Append new quantity selector then remove original
        el.after(template(data)).remove();
      });

      // Setup listeners to add/subtract from the input
      $('.js--qty-adjuster').on('click', function() {
        var el = $(this),
            id = el.data('id'),
            qtySelector = el.siblings('.js--num'),
            qty = parseInt( qtySelector.val() );

        var qty = validateQty(qty);
        qtyMin = $body.hasClass('template-product') ? 1 : qtyMin;

        // Add or subtract from the current quantity
        if (el.hasClass('js--add')) {
          qty = qty + 1;
        } else {
          qty = qty <= qtyMin ? qtyMin : qty - 1;
        }

        // Update the input's number
        qtySelector.val(qty);
      });

    }
  };

  scrollTop = function () {
    if ($body.scrollTop() > 0 || $html.scrollTop() > 0) {
      $('html, body').animate({
        scrollTop: 0
      }, 250, 'swing');
    }
  };

  toggleCallback = function (data) {
    // Run the callback if it's a function
    if (typeof settings.onToggleCallback == 'function') {
      settings.onToggleCallback.call(this, data);
    }
  };

  validateQty = function (qty) {
    if((parseFloat(qty) == parseInt(qty)) && !isNaN(qty)) {
      // We have a valid number!
      return qty;
    } else {
      // Not a number. Default to 1.
      return 1;
    }
  };

  module = {
    init: init
  };

  return module;

}(ajaxifyShopify || {}, jQuery));



/* ================ SLATE ================ */
window.theme = window.theme || {};

theme.Sections = function Sections() {
  this.constructors = {};
  this.instances = [];

  $(document)
    .on('shopify:section:load', this._onSectionLoad.bind(this))
    .on('shopify:section:unload', this._onSectionUnload.bind(this))
    .on('shopify:section:select', this._onSelect.bind(this))
    .on('shopify:section:deselect', this._onDeselect.bind(this))
    .on('shopify:block:select', this._onBlockSelect.bind(this))
    .on('shopify:block:deselect', this._onBlockDeselect.bind(this));
};

theme.Sections.prototype = _.assignIn({}, theme.Sections.prototype, {
  _createInstance: function(container, constructor) {
    var $container = $(container);
    var id = $container.attr('data-section-id');
    var type = $container.attr('data-section-type');

    constructor = constructor || this.constructors[type];

    if (_.isUndefined(constructor)) {
      return;
    }

    var instance = _.assignIn(new constructor(container), {
      id: id,
      type: type,
      container: container
    });

    this.instances.push(instance);
  },

  _onSectionLoad: function(evt) {
    var container = $('[data-section-id]', evt.target)[0];
    if (container) {
      this._createInstance(container);
    }
  },

  _onSectionUnload: function(evt) {
    this.instances = _.filter(this.instances, function(instance) {
      var isEventInstance = instance.id === evt.detail.sectionId;

      if (isEventInstance) {
        if (_.isFunction(instance.onUnload)) {
          instance.onUnload(evt);
        }
      }

      return !isEventInstance;
    });
  },

  _onSelect: function(evt) {
    // eslint-disable-next-line no-shadow
    var instance = _.find(this.instances, function(instance) {
      return instance.id === evt.detail.sectionId;
    });

    if (!_.isUndefined(instance) && _.isFunction(instance.onSelect)) {
      instance.onSelect(evt);
    }
  },

  _onDeselect: function(evt) {
    // eslint-disable-next-line no-shadow
    var instance = _.find(this.instances, function(instance) {
      return instance.id === evt.detail.sectionId;
    });

    if (!_.isUndefined(instance) && _.isFunction(instance.onDeselect)) {
      instance.onDeselect(evt);
    }
  },

  _onBlockSelect: function(evt) {
    // eslint-disable-next-line no-shadow
    var instance = _.find(this.instances, function(instance) {
      return instance.id === evt.detail.sectionId;
    });

    if (!_.isUndefined(instance) && _.isFunction(instance.onBlockSelect)) {
      instance.onBlockSelect(evt);
    }
  },

  _onBlockDeselect: function(evt) {
    // eslint-disable-next-line no-shadow
    var instance = _.find(this.instances, function(instance) {
      return instance.id === evt.detail.sectionId;
    });

    if (!_.isUndefined(instance) && _.isFunction(instance.onBlockDeselect)) {
      instance.onBlockDeselect(evt);
    }
  },

  register: function(type, constructor) {
    this.constructors[type] = constructor;

    $('[data-section-type=' + type + ']').each(
      function(index, container) {
        this._createInstance(container, constructor);
      }.bind(this)
    );
  }
});

/**
 * A11y Helpers
 * -----------------------------------------------------------------------------
 * A collection of useful functions that help make your theme more accessible
 * to users with visual impairments.
 */

theme.a11y = {
  /**
   * For use when focus shifts to a container rather than a link
   * eg for In-page links, after scroll, focus shifts to content area so that
   * next `tab` is where user expects if focusing a link, just $link.focus();
   *
   * @param {JQuery} $element - The element to be acted upon
   */
  pageLinkFocus: function($element) {
    var focusClass = 'js-focus-hidden';

    $element
      .first()
      .attr('tabIndex', '-1')
      .focus()
      .addClass(focusClass)
      .one('blur', callback);

    function callback() {
      $element
        .first()
        .removeClass(focusClass)
        .removeAttr('tabindex');
    }
  },

  /**
   * If there's a hash in the url, focus the appropriate element
   */
  focusHash: function() {
    var hash = window.location.hash;

    // is there a hash in the url? is it an element on the page?
    if (hash && document.getElementById(hash.slice(1))) {
      this.pageLinkFocus($(hash));
    }
  },

  /**
   * When an in-page (url w/hash) link is clicked, focus the appropriate element
   */
  bindInPageLinks: function() {
    $('a[href*=#]').on(
      'click',
      function(evt) {
        this.pageLinkFocus($(evt.currentTarget.hash));
      }.bind(this)
    );
  },

  /**
   * Traps the focus in a particular container
   *
   * @param {object} options - Options to be used
   * @param {jQuery} options.$container - Container to trap focus within
   * @param {jQuery} options.$elementToFocus - Element to be focused when focus leaves container
   * @param {string} options.namespace - Namespace used for new focus event handler
   */
  trapFocus: function(options) {
    var eventName = options.namespace
      ? 'focusin.' + options.namespace
      : 'focusin';

    if (!options.$elementToFocus) {
      options.$elementToFocus = options.$container;
    }

    options.$container.attr('tabindex', '-1');
    options.$elementToFocus.focus();

    $(document).on(eventName, function(evt) {
      if (
        options.$container[0] !== evt.target &&
        !options.$container.has(evt.target).length
      ) {
        options.$container.focus();
      }
    });
  },

  /**
   * Removes the trap of focus in a particular container
   *
   * @param {object} options - Options to be used
   * @param {jQuery} options.$container - Container to trap focus within
   * @param {string} options.namespace - Namespace used for new focus event handler
   */
  removeTrapFocus: function(options) {
    var eventName = options.namespace
      ? 'focusin.' + options.namespace
      : 'focusin';

    if (options.$container && options.$container.length) {
      options.$container.removeAttr('tabindex');
    }

    $(document).off(eventName);
  }
};


/* ================ MODULES ================ */
/* eslint-disable no-new */
window.timber = window.timber || {};

timber.cacheSelectors = function () {
  timber.cache = {
    // General
    $html: $('html'),
    $body: $('body'),
    $window: $(window),
    $breadcrumbs: $('.breadcrumb'),

    // Navigation
    $navigation: $('#AccessibleNav'),
    $mobileNav: $('#MobileNav'),
    $hasDropdownItem: $('.site-nav--has-dropdown'),
    $menuToggle: $('.menu-toggle'),

    // Product Page
    $productImageWrap: $('#productPhoto'),
    $productImage: $('#productPhotoImg'),
    $thumbImages: $('#productThumbs').find('a.product-photo-thumb'),
    $shareButtons: $('.social-sharing'),

    // Collection Pages
    $collectionFilters: $('#collectionFilters'),
    $advancedFilters: $('.advanced-filters'),
    $toggleFilterBtn: $('#toggleFilters'),

    // Cart Pages
    $emptyCart: $('#EmptyCart'),
    $ajaxCartContainer: $('#ajaxifyCart'),
    cartNoCookies: 'cart--no-cookies',

    // Equal height elements
    $featuredBoxImages: $('.featured-box--inner'),
    $featuredBoxTitles: $('.featured-box--title')
  };
};

timber.cacheVariables = function () {
  timber.vars = {
    // Breakpoints (from timber.scss.liquid)
    bpLarge: 769,

    // MediaQueries (from timber.scss.liquid)
    mediaQueryLarge: 'screen and (min-width: 769px)',
    isLargeBp: false,
    isTouch: timber.cache.$html.hasClass('supports-touch')
  }
};

timber.init = function () {
  timber.cacheSelectors();
  timber.cacheVariables();

  timber.cache.$html.removeClass('no-js').addClass('js');
  if ('ontouchstart' in window) {
    timber.cache.$html.removeClass('no-touch').addClass('touch');
  }

  timber.initCart();
  // timber.equalHeights();
  timber.responsiveVideos();
  timber.toggleFilters();

  
};

timber.mobileNav = function () {
  var classes = {
    active: 'nav-active',
    dropdownButton: 'mobile-nav--button'
  }

  var selectors = {
    parentLink: '[data-meganav-type="parent"]',
    dropdownButton: '.' + classes.dropdownButton
  }

  var $mobileNav = timber.cache.$mobileNav,
    $mobileNavBtn = $mobileNav.find(selectors.dropdownButton);

  $mobileNavBtn.on('click', function (evt) {
    var $el = $(this);
    var $parentLink = $el.closest('li');

    if (!$el.hasClass(classes.active)) {
      showDropdown($el, $parentLink);
      return;
    }

    if ($el.hasClass(classes.active)) {
      hideDropdowns($el, $parentLink);
      return;
    }
  });

  function showDropdown($el, $dropdown) {
    $el.addClass(classes.active)
    var $parent = $dropdown.find('> ' + selectors.parentLink);

    $dropdown.addClass(classes.active);
    $el.attr('aria-expanded', 'true');
  }

  function hideDropdowns($el, $parentLink) {
    $el.removeClass(classes.active)
    $parentLink.removeClass(classes.active)

    $.each($parentLink, function () {
      var $dropdown = $(this),
        $parent = $dropdown.find('> ' + selectors.parentLink);
      $dropdown.removeClass(classes.active);
      $el.attr('aria-expanded', 'false');
    })
  }
}

timber.accessibleNav = function () {

  var classes = {
    active: 'nav-hover',
    focus: 'nav-focus',
    outside: 'nav-outside',
    hasDropdown: 'site-nav--has-dropdown',
    link: 'site-nav--link'
  }
  var selectors = {
    active: '.' + classes.active,
    hasDropdown: '.' + classes.hasDropdown,
    dropdown: '[data-meganav-dropdown]',
    link: '.' + classes.link,
    nextLink: '> .' + classes.link,
    parentLink: '[data-meganav-type="parent"]',
    childLink: '[data-meganav-type="child"]'
  }

  var $nav = timber.cache.$navigation,
    $allLinks = $nav.find(selectors.link),
    $parents = $nav.find(selectors.hasDropdown),
    $childLinks = $nav.find(selectors.childLink),
    $topLevel = $parents.find(selectors.nextLink),
    $dropdowns = $nav.find(selectors.dropdown),
    $subMenuLinks = $dropdowns.find(selectors.link);

  // Mouseenter
  $parents.on('mouseenter touchstart', function (evt) {

    var $el = $(this);
    var evtType = evt.type;
    var $dropdowns = $nav.find(selectors.active);

    if (!$el.hasClass(classes.active)) {
      // force stop the click from happening
      evt.preventDefault();
      evt.stopImmediatePropagation();
    }

    // Make sure we close any opened same level dropdown before opening a new one
    if (evtType === 'touchstart' && $dropdowns.length > 0) { hideDropdown($el); }

    showDropdown($el);

  });

  $childLinks.on('touchstart', function (evt) {
    evt.stopImmediatePropagation();
  });

  $parents.on('mouseleave', function () {
    hideDropdown($(this));
  });

  $allLinks.on('focus', function () {
    handleFocus($(this));
  })

  $allLinks.on('blur', function () {
    removeFocus($topLevel);
  })

  // accessibleNav private methods
  function handleFocus($el) {
    var $newFocus = null,
      $previousItem = $el.parent().prev();

    // Always put tabindex -1 on previous element just in case the user is going backward.
    // In that case, we want to focus on the previous parent and not the previous parent childs

    $allLinks.attr('tabindex', '');

    if ($previousItem.hasClass(classes.hasDropdown)) {
      $previousItem.find(selectors.dropdown + ' ' + selectors.link).attr('tabindex', -1);
    }

    $newFocus = $el.parents(selectors.hasDropdown).find('> ' + selectors.link);
    addFocus($newFocus);

  }

  function showDropdown($el) {
    var $toplevel = $el.find(selectors.nextLink);

    $toplevel.attr('aria-expanded', true);

    $el.addClass(classes.active);

    setTimeout(function () {
      timber.cache.$body.on('touchstart.MegaNav', function () {
        hideDropdowns();
      });
    }, 250);
  }

  function hideDropdown($el) {
    var $dropdowns = $el.parent().find(selectors.active);
    var $parentLink = $dropdowns.find(selectors.nextLink);

    $parentLink.attr('aria-expanded', false);

    $dropdowns.removeClass(classes.active);

    timber.cache.$body.off('touchstart.MegaNav');
  }

  function hideDropdowns() {
    var $dropdowns = $nav.find(selectors.active);
    $.each($dropdowns, function () {
      hideDropdown($(this));
    });
  }

  function addFocus($el) {
    $el.addClass(classes.focus);

    if ($el.attr('aria-expanded') !== undefined) {
      $el.attr('aria-expanded', true);
    }
  }

  function removeFocus($el) {
    $el.removeClass(classes.focus);

    $subMenuLinks.attr('tabindex', -1);

    if ($el.attr('aria-expanded') !== undefined) {
      $el.attr('aria-expanded', false);
    }
  }

  // Check if dropdown is outside of viewport
  function handleDropdownOffset($dropdowns) {
    var viewportSize = $(window).width();
    $dropdowns.removeClass(classes.outside);

    $.each($dropdowns, function () {
      $dropdown = $(this);
      var dropdownOffset = $dropdown.offset().left + $dropdown.width();
      if (dropdownOffset > viewportSize) {
        $dropdown.addClass(classes.outside);
      }
    });
  }

  timber.cache.$window.load(function () {
    handleDropdownOffset($dropdowns);
  });

  timber.cache.$window.resize(function () {
    afterResize(function () {
      handleDropdownOffset($dropdowns);
    }, 250);
  });
};

timber.responsiveNav = function () {
  $(window).resize(function () {
    afterResize(function () {
      // Replace original nav items and remove more link
      timber.cache.$navigation.append($('#moreMenu--list').html());
      $('#moreMenu').remove();
      timber.alignMenu();
      timber.accessibleNav();
    }, 200, 'uniqueID');
  });
  timber.alignMenu();
  timber.accessibleNav();
  timber.mobileNav();
};

timber.alignMenu = function () {
  var $nav = timber.cache.$navigation,
    w = 0,
    i = 0;
  wrapperWidth = $nav.outerWidth() - 101,
    menuhtml = '';

  if (window.innerWidth < timber.vars.bpLarge) {
    return;
  }

  $.each($nav.children(), function () {
    var $el = $(this);

    // Ignore hidden customer links (for mobile)
    if (!$el.hasClass('large-hide')) {
      w += $el.outerWidth(true);
    }

    if (wrapperWidth < w) {
      menuhtml += $('<div>').append($el.clone()).html();
      $el.remove();

      // Ignore hidden customer links (for mobile)
      if (!$el.hasClass('large-hide')) {
        i++;
      }
    }
  });

  if (wrapperWidth < w) {
    $nav.append(
      '<li id="moreMenu" class="site-nav--has-dropdown">'
      + '<button class="site-nav--link" data-meganav-type="parent" aria-expanded="false">' + theme.strings.navigation.more_link + '<span class="icon icon-arrow-down" aria-hidden="true"></span></button>'
      + '<ul id="moreMenu--list" class="site-nav--dropdown site-nav--has-grandchildren site-nav--dropdown--more">' + menuhtml + '</ul></li>'
    );

    $('#moreMenu').find('a').attr('tabindex', '-1');

    if (i <= 1) {
      // Bail, and replace original nav items
      timber.cache.$navigation.append($('#moreMenu--list').html());
      $('#moreMenu').remove();
    }
  }
};

timber.toggleMenu = function () {
  var $mainHeader = $('#shopify-section-header');
  var $navBar = $('#navBar');
  var $siteHeader = $mainHeader.find('.site-header');
  var showNavClass = 'show-nav';
  var hiddenClass = 'site-header--hidden';

  timber.cache.$menuToggle.on('click', function () {
    var $el = $(this),
    isExpanded = ($el.attr('aria-expanded') === 'true');

    timber.cache.$html.toggleClass(showNavClass);

    $el.attr('aria-expanded', !isExpanded);

    if (!isExpanded) {
      setTimeout(function () {
        $siteHeader.addClass(hiddenClass);
      }, 450); // Match CSS transition speed
      theme.a11y.trapFocus({
        $container: $mainHeader,
        $elementToFocus: $('#MobileNav > li:first-child a'),
        namespace: 'mobileMenuToggle'
      });
       $navBar.scrollTop(0);
    } else {
      $siteHeader.removeClass(hiddenClass);
      theme.a11y.removeTrapFocus({
        $container: $mainHeader,
        namespace: 'mobileMenuToggle'
      });
    }

    // Close ajax cart if open (keep selectors live, modal is inserted with JS)
    if ($('#ajaxifyModal').hasClass('is-visible')) {
      $('#ajaxifyModal').removeClass('is-visible');
      timber.cache.$html.addClass(showNavClass);
    }
  });
};

timber.initCart = function() {
  if (theme.settings.cartType != 'page'){
    ajaxifyShopify.init({
      method: theme.settings.cartType,
      wrapperClass: 'wrapper',
      formSelector: '[data-product-form]',
      addToCartSelector: '#addToCart',
      cartCountSelector: '.cart-count',
      toggleCartButton: '.cart-toggle',
      useCartTemplate: true,
      btnClass: 'btn',
      moneyFormat: moneyFormat,
      disableAjaxCart: false,
      enableQtySelectors: true
    });
  }

  if (!timber.cookiesEnabled()) {
    timber.cache.$emptyCart.addClass(timber.cache.cartNoCookies);
    timber.cache.$ajaxCartContainer.addClass(timber.cache.cartNoCookies);
  }
};

timber.cookiesEnabled = function () {
  var cookieEnabled = navigator.cookieEnabled;

  if (!cookieEnabled) {
    document.cookie = 'testcookie';
    cookieEnabled = (document.cookie.indexOf('testcookie') !== -1);
  }
  return cookieEnabled;
};

timber.equalHeights = function (el) {
  $(window).load(function () {
    timber.resizeElements(this);
  });

  $(window).resize(function (el) {
    afterResize(function () {
      timber.resizeElements(this);
    }, 250, 'id');
  });

  timber.resizeElements(this);
};

timber.resizeElements = function ($container, id) {
  var $id = $container.attr('data-section-id', id);
  var $grid = $container.find('.grid-uniform');
  var $gridImages = $id.find('.product-grid-image');

  $gridImages.css('height', 'auto').equalHeights(this);

  var $featuredBoxImages = $container.find('.featured-box--inner');
  var $featuredBoxTitles = $container.find('.featured-box--title');

  $featuredBoxImages.css('height', 'auto').equalHeights(this);
  $featuredBoxTitles.css('height', 'auto').equalHeights(this);
};

timber.responsiveVideos = function () {
  var $iframeVideo = $('iframe[src*="youtube.com/embed"], iframe[src*="player.vimeo"]');
  var $iframeReset = $iframeVideo.add('iframe#admin_bar_iframe');

  $iframeVideo.each(function () {
    // Add wrapper to make video responsive but not for video sections
    if (!$(this).parent('div.video-wrapper').length) {
      $(this).wrap('<div class="video-wrapper"></div>');
    };
  });

  $iframeReset.each(function () {
    // Re-set the src attribute on each iframe after page load
    // for Chrome's 'incorrect iFrame content on 'back'' bug.
    // https://code.google.com/p/chromium/issues/detail?id=395791
    // Need to specifically target video and admin bar
    this.src = this.src;
  });
};

timber.toggleFilters = function () {
  if (timber.cache.$collectionFilters.length) {
    timber.cache.$toggleFilterBtn.on('click', function () {
      timber.cache.$toggleFilterBtn.toggleClass('is-active');
      timber.cache.$collectionFilters.slideToggle(200);

      // Scroll to top of filters if user is down the page a bit
      if ($(window).scrollTop() > timber.cache.$breadcrumbs.offset().top) {
        $('html, body').animate({
          scrollTop: timber.cache.$breadcrumbs.offset().top
        });
      }
    });
  }
};

timber.sortFilters = function () {
  timber.cache.$advancedFilters.each(function () {
    var $el = $(this),
      $tags = $el.find('li'),
      aNumber = /\d+/,
      sorted = false;
    $tags.sort(function (a, b) {
      a = parseInt(aNumber.exec($(a).text()), 10);
      b = parseInt(aNumber.exec($(b).text()), 10);
      if (isNaN(a) || isNaN(b)) {
        return;
      }
      else {
        sorted = true;
        return a - b;
      }
    });
    if (sorted) {
      $el.append($tags);
    }
  });
};

timber.formatMoney = function (val) {

  

  

return val;
};

timber.formatSaleTag = function (val) {
  // If not using multiple currencies
  if (moneyFormat.indexOf('money') === -1) {
    // If we use amount
    if ( (moneyFormat.replace(/\s/g, '').indexOf('{{amount}}') > -1) && (moneyFormat.indexOf('.') === -1) ) {
      // If there are no cents, remove decimals
      if ( val.indexOf('.00') > -1 ) {
        return val.replace('.00', '')
      }
    }
    // If we use amount_with_comma_separator
    else if (moneyFormat.replace(/\s/g, '').indexOf('{{amount_with_comma_separator}}') > -1) {
      // If there are no cents, remove decimals
      if ( val.indexOf(',00') > -1 ) {
        return val.replace(',00', '')
      }
    }
  }
  return val;
};

// Initialize Timber's JS on docready
$(timber.init)

/*!
 * imagesLoaded PACKAGED v4.1.1
 * JavaScript is all like "You images are done yet or what?"
 * MIT License
 */

!function(t,e){"function"==typeof define&&define.amd?define("ev-emitter/ev-emitter",e):"object"==typeof module&&module.exports?module.exports=e():t.EvEmitter=e()}("undefined"!=typeof window?window:this,function(){function t(){}var e=t.prototype;return e.on=function(t,e){if(t&&e){var i=this._events=this._events||{},n=i[t]=i[t]||[];return-1==n.indexOf(e)&&n.push(e),this}},e.once=function(t,e){if(t&&e){this.on(t,e);var i=this._onceEvents=this._onceEvents||{},n=i[t]=i[t]||{};return n[e]=!0,this}},e.off=function(t,e){var i=this._events&&this._events[t];if(i&&i.length){var n=i.indexOf(e);return-1!=n&&i.splice(n,1),this}},e.emitEvent=function(t,e){var i=this._events&&this._events[t];if(i&&i.length){var n=0,o=i[n];e=e||[];for(var r=this._onceEvents&&this._onceEvents[t];o;){var s=r&&r[o];s&&(this.off(t,o),delete r[o]),o.apply(this,e),n+=s?0:1,o=i[n]}return this}},t}),function(t,e){"use strict";"function"==typeof define&&define.amd?define(["ev-emitter/ev-emitter"],function(i){return e(t,i)}):"object"==typeof module&&module.exports?module.exports=e(t,require("ev-emitter")):t.imagesLoaded=e(t,t.EvEmitter)}(window,function(t,e){function i(t,e){for(var i in e)t[i]=e[i];return t}function n(t){var e=[];if(Array.isArray(t))e=t;else if("number"==typeof t.length)for(var i=0;i<t.length;i++)e.push(t[i]);else e.push(t);return e}function o(t,e,r){return this instanceof o?("string"==typeof t&&(t=document.querySelectorAll(t)),this.elements=n(t),this.options=i({},this.options),"function"==typeof e?r=e:i(this.options,e),r&&this.on("always",r),this.getImages(),h&&(this.jqDeferred=new h.Deferred),void setTimeout(function(){this.check()}.bind(this))):new o(t,e,r)}function r(t){this.img=t}function s(t,e){this.url=t,this.element=e,this.img=new Image}var h=t.jQuery,a=t.console;o.prototype=Object.create(e.prototype),o.prototype.options={},o.prototype.getImages=function(){this.images=[],this.elements.forEach(this.addElementImages,this)},o.prototype.addElementImages=function(t){"IMG"==t.nodeName&&this.addImage(t),this.options.background===!0&&this.addElementBackgroundImages(t);var e=t.nodeType;if(e&&d[e]){for(var i=t.querySelectorAll("img"),n=0;n<i.length;n++){var o=i[n];this.addImage(o)}if("string"==typeof this.options.background){var r=t.querySelectorAll(this.options.background);for(n=0;n<r.length;n++){var s=r[n];this.addElementBackgroundImages(s)}}}};var d={1:!0,9:!0,11:!0};return o.prototype.addElementBackgroundImages=function(t){var e=getComputedStyle(t);if(e)for(var i=/url\((['"])?(.*?)\1\)/gi,n=i.exec(e.backgroundImage);null!==n;){var o=n&&n[2];o&&this.addBackground(o,t),n=i.exec(e.backgroundImage)}},o.prototype.addImage=function(t){var e=new r(t);this.images.push(e)},o.prototype.addBackground=function(t,e){var i=new s(t,e);this.images.push(i)},o.prototype.check=function(){function t(t,i,n){setTimeout(function(){e.progress(t,i,n)})}var e=this;return this.progressedCount=0,this.hasAnyBroken=!1,this.images.length?void this.images.forEach(function(e){e.once("progress",t),e.check()}):void this.complete()},o.prototype.progress=function(t,e,i){this.progressedCount++,this.hasAnyBroken=this.hasAnyBroken||!t.isLoaded,this.emitEvent("progress",[this,t,e]),this.jqDeferred&&this.jqDeferred.notify&&this.jqDeferred.notify(this,t),this.progressedCount==this.images.length&&this.complete(),this.options.debug&&a&&a.log("progress: "+i,t,e)},o.prototype.complete=function(){var t=this.hasAnyBroken?"fail":"done";if(this.isComplete=!0,this.emitEvent(t,[this]),this.emitEvent("always",[this]),this.jqDeferred){var e=this.hasAnyBroken?"reject":"resolve";this.jqDeferred[e](this)}},r.prototype=Object.create(e.prototype),r.prototype.check=function(){var t=this.getIsImageComplete();return t?void this.confirm(0!==this.img.naturalWidth,"naturalWidth"):(this.proxyImage=new Image,this.proxyImage.addEventListener("load",this),this.proxyImage.addEventListener("error",this),this.img.addEventListener("load",this),this.img.addEventListener("error",this),void(this.proxyImage.src=this.img.src))},r.prototype.getIsImageComplete=function(){return this.img.complete&&void 0!==this.img.naturalWidth},r.prototype.confirm=function(t,e){this.isLoaded=t,this.emitEvent("progress",[this,this.img,e])},r.prototype.handleEvent=function(t){var e="on"+t.type;this[e]&&this[e](t)},r.prototype.onload=function(){this.confirm(!0,"onload"),this.unbindEvents()},r.prototype.onerror=function(){this.confirm(!1,"onerror"),this.unbindEvents()},r.prototype.unbindEvents=function(){this.proxyImage.removeEventListener("load",this),this.proxyImage.removeEventListener("error",this),this.img.removeEventListener("load",this),this.img.removeEventListener("error",this)},s.prototype=Object.create(r.prototype),s.prototype.check=function(){this.img.addEventListener("load",this),this.img.addEventListener("error",this),this.img.src=this.url;var t=this.getIsImageComplete();t&&(this.confirm(0!==this.img.naturalWidth,"naturalWidth"),this.unbindEvents())},s.prototype.unbindEvents=function(){this.img.removeEventListener("load",this),this.img.removeEventListener("error",this)},s.prototype.confirm=function(t,e){this.isLoaded=t,this.emitEvent("progress",[this,this.element,e])},o.makeJQueryPlugin=function(e){e=e||t.jQuery,e&&(h=e,h.fn.imagesLoaded=function(t,e){var i=new o(this,t,e);return i.jqDeferred.promise(h(this))})},o.makeJQueryPlugin(),o});


/* ================ SECTIONS ================ */
window.theme = window.theme || {};

theme.FeaturedCollections = (function() {
  function FeaturedCollections(container) {
    var $container = (this.$container = $(container));
    timber.cacheSelectors();
    timber.resizeElements($container);

    $(window).resize(function() {
      timber.resizeElements($container);
    });
  }

  return FeaturedCollections;
})();

window.theme = window.theme || {};

theme.CollectionRows = (function() {
  function CollectionRows(container) {
    var $container = (this.$container = $(container));
    var id = (this.id = $container.attr('data-section-id'));
    timber.cacheSelectors();
    timber.resizeElements($container, id);
    $(window).resize(function() {
      timber.resizeElements($container, id);
    });
  }

  return CollectionRows;
})();

window.theme = window.theme || {};

theme.Collection = (function() {
  function Collection(container) {
    var $container = (this.$container = $(container));
    var id = (this.id = $container.attr('data-section-id'));
    timber.cacheSelectors();
    timber.resizeElements($container, id);
    $(window).resize(function() {
      timber.resizeElements($container, id);
    });
  }

  return Collection;
})();

window.theme = window.theme || {};

theme.HeaderSection = (function() {
  function Header() {
    timber.cacheSelectors();
    timber.toggleMenu();

    $(window)
      .on('load', timber.responsiveNav)
      .resize();
  }

  return Header;
})();

window.theme = window.theme || {};

theme.ListCollections = (function() {
  function ListCollections(container) {
    var $container = (this.$container = $(container));
    timber.cacheSelectors();
    timber.resizeElements($container);

    $(window).resize(function() {
      timber.resizeElements($container);
    });
  }

  return ListCollections;
})();

theme.Maps = (function() {
  var config = {
    zoom: 14
  };
  var apiStatus = null;
  var mapsToLoad = [];

  function Map(container) {
    theme.$currentMapContainer = this.$container = $(container);
    var key = this.$container.data('api-key');

    if (typeof key !== 'string' || key === '') {
      return;
    }

    if (apiStatus === 'loaded') {
      var self = this;

      // Check if the script has previously been loaded with this key
      var $script = $('script[src*="' + key + '&"]');
      if ($script.length === 0) {
        $.getScript('https://maps.googleapis.com/maps/api/js?key=' + key).then(
          function() {
            apiStatus = 'loaded';
            self.createMap();
          }
        );
      } else {
        this.createMap();
      }
    } else {
      mapsToLoad.push(this);

      if (apiStatus !== 'loading') {
        apiStatus = 'loading';
        if (typeof window.google === 'undefined') {
          $.getScript(
            'https://maps.googleapis.com/maps/api/js?key=' + key
          ).then(function() {
            apiStatus = 'loaded';
            initAllMaps();
          });
        }
      }
    }
  }

  function initAllMaps() {
    // API has loaded, load all Map instances in queue
    $.each(mapsToLoad, function(index, instance) {
      instance.createMap();
    });
  }

  function geolocate($map) {
    var deferred = $.Deferred();
    var geocoder = new google.maps.Geocoder();
    var address = $map.data('address-setting');

    geocoder.geocode({ address: address }, function(results, status) {
      if (status !== google.maps.GeocoderStatus.OK) {
        deferred.reject(status);
      }

      deferred.resolve(results);
    });

    return deferred;
  }

  Map.prototype = _.assignIn({}, Map.prototype, {
    createMap: function() {
      var $map = this.$container.find('.map-section__container');

      return geolocate($map)
        .then(
          function(results) {
            var mapOptions = {
              zoom: config.zoom,
              styles: config.styles,
              center: results[0].geometry.location,
              draggable: false,
              clickableIcons: false,
              scrollwheel: false,
              disableDoubleClickZoom: true,
              disableDefaultUI: true
            };

            var map = (this.map = new google.maps.Map($map[0], mapOptions));
            var center = (this.center = map.getCenter());

            //eslint-disable-next-line no-unused-vars
            var marker = new google.maps.Marker({
              map: map,
              position: center
            });

            google.maps.event.addDomListener(window, 'resize', function() {
              google.maps.event.trigger(map, 'resize');
              map.setCenter(center);
            });
          }.bind(this)
        )
        .fail(function() {
          var errorMessage;

          switch (status) {
            case 'ZERO_RESULTS':
              errorMessage = theme.strings.map.addressNoResults;
              break;
            case 'OVER_QUERY_LIMIT':
              errorMessage = theme.strings.map.addressQueryLimit;
              break;
            default:
              errorMessage = theme.strings.map.addressError;
              break;
          }

          // Only show error in the theme editor
          if (Shopify.designMode) {
            var $mapContainer = $map.parents('.map-section');

            $mapContainer.addClass('page-width map-section--load-error');
            $mapContainer.find('.map-section__content-wrapper').remove();
            $mapContainer
              .find('.map-section__wrapper')
              .html(
                '<div class="errors text-center" style="width: 100%;">' +
                  errorMessage +
                  '</div>'
              );
          }
        });
    },

    onUnload: function() {
      if (typeof window.google !== 'undefined') {
        google.maps.event.clearListeners(this.map, 'resize');
      }
    }
  });

  return Map;
})();

// Global function called by Google on auth errors.
// Show an auto error message on all map instances.
// eslint-disable-next-line camelcase, no-unused-vars
function gm_authFailure() {
  if (!Shopify.designMode) return;

  theme.$currentMapContainer.addClass('page-width map-section--load-error');
  theme.$currentMapContainer.find('.map-section__content-wrapper').remove();
  theme.$currentMapContainer
    .find('.map-section__wrapper')
    .html(
      '<div class="errors text-center" style="width: 100%;">' +
        theme.strings.map.authError +
        '</div>'
    );
}

/* eslint-disable no-new */
theme.Product = (function() {
  var defaults = {
    selectors: {
      addToCart: '#addToCart',
      productPrice: '#productPrice',
      comparePrice: '#comparePrice',
      addToCartText: '#addToCartText',
      quantityElements: '.quantity-selector, label + .js-qty',
      optionSelector: 'productSelect'
    }
  };

  function Product(container) {
    var $container = this.$container = $(container);
    var sectionId = this.sectionId = $container.attr('data-section-id');

    this.settings = $.extend({}, defaults, {
      sectionId: sectionId,
      enableHistoryState: true,
      showComparePrice: $container.attr('data-show-compare-at-price'),
      ajaxCartMethod: $container.attr('data-ajax-cart-method'),
      stockSetting: $container.attr('data-stock'),
      incomingMessage: $container.attr('data-incoming-transfer'),
      selectors: {
        unitPriceContainer: '[data-unit-price-container]',
        unitPrice: '[data-unit-price]',
        unitPriceBaseUnit: '[data-unit-price-base-unit]',
        priceContainer: '[data-price]',
        originalSelectorId: 'productSelect-' + sectionId,
        $addToCart: $('#addToCart-' + sectionId),
        $SKU: $('.variant-sku', this.$container),
        $productPrice: $('#productPrice-' + sectionId),
        $comparePrice: $('#comparePrice-' + sectionId),
        $addToCartText: $('#addToCartText-' + sectionId),
        $quantityElements: $('#quantity-selector-' + sectionId),
        $variantQuantity: $('#variantQuantity-' + sectionId),
        $variantQuantityMessage: $('#variantQuantity-' + sectionId + '__message'),
        $variantIncoming: $('#variantIncoming-' + sectionId),
        $variantIncomingMessage: $('#variantIncoming-' + sectionId + '__message'),
        $productImageContainer: $('#productPhotoContainer-' + sectionId),
        $productImageWrapper: $('[id^="productPhotoWrapper-' + sectionId + '"]'),
        $productImage: $('[id^="productPhotoImg-' + sectionId + '"]'),
        $productFullDetails: $('.full-details', this.$container),
        $thumbImages: $('#productThumbs-' + sectionId).find('a.product-photo-thumb'),
        $shopifyPaymentButton: '.shopify-payment-button'
      }
    });

    // disable history state if on homepage
    if($('body').hasClass('template-index')) {
      this.settings.enableHistoryState = false;
    }

    // Stop parsing if we don't have the product json script tag when loading
    // section in the Theme Editor
    if (!$('#ProductJson-' + sectionId).html()) {
      return;
    }

    this.zoomEnabled = $container.attr('data-zoom-enabled');

    // this.productSingleObject = JSON.parse(document.getElementById('ProductJson-' + sectionId).innerHTML);
    this.productSingleObject = JSON.parse($('#ProductJson-' + sectionId).html());
    this.addVariantInfo();
    this.init();

    // Pre-loading product images to avoid a lag when a thumbnail is clicked, or
    // when a variant is selected that has a variant image
    Shopify.Image.preload(this.productSingleObject.images);

    if (this.settings.ajaxCartMethod != 'page') {
      ajaxifyShopify.init({
        method: 'modal',
        wrapperClass: 'wrapper',
        formSelector: '[data-product-form]',
        addToCartSelector: '#addToCart-' + sectionId,
        cartCountSelector: '.cart-count',
        toggleCartButton: '.cart-toggle',
        useCartTemplate: true,
        btnClass: 'btn',
        moneyFormat: moneyFormat,
        disableAjaxCart: false,
        enableQtySelectors: true
      });
    }
  }

  Product.prototype = _.assignIn({}, Product.prototype, {
    init: function() {
      this.initProductVariant();
      this.addQuantityButtons();
      this.productImageSwitch();
      this.initBreakpoints();

      if (timber.vars.isLargeBp && this.zoomEnabled) {
        productImageZoom();
      }
    },

    onUnload: function() {
      this.$container.off(this.settings.sectionId);
    },

    addVariantInfo: function() {
      if (!this.productSingleObject) {
        return;
      }

      if (this.settings.stockSetting === 'false' && this.settings.incomingMessage === 'false'){
        return;
      }

      var variantInfo = JSON.parse($('#VariantJson-' + this.settings.sectionId, this.$container).html());
      for (var i = 0; i < variantInfo.length; i++) {
        $.extend(this.productSingleObject.variants[i], variantInfo[i]);
      }
    },

    addQuantityButtons: function(){
      if (this.settings.selectors.$quantityElements){
        this.settings.selectors.$quantityElements.show();
        
          this.qtySelectors();
        
      }

    },

    qtySelectors: function() {

      validateQty = function (qty) {
        if((parseFloat(qty) == parseInt(qty)) && !isNaN(qty)) {
          // We have a valid number!
          return qty;
        } else {
          // Not a number. Default to 1.
          return 1;
        }
      };

      // Change number inputs to JS ones, similar to ajax cart but without API integration.
      // Make sure to add the existing name and id to the new input element
      var numInputs = $('input[type="number"]', this.$container);

      // Qty selector has a minimum of 1 on the product page
      // and 0 in the cart (determined on qty click)
      var qtyMin = 0;

      if (numInputs.length) {
        numInputs.each(function() {
          var el = $(this),
          currentQty = parseInt(el.val()),
          inputName = el.attr('name'),
          inputId = el.attr('id');

          var itemAdd = currentQty + 1,
          itemMinus = currentQty - 1,
          itemQty = currentQty;

          var source = $("#jsQty").html(),
          template = Handlebars.compile(source),
          data = {
            key: el.data('id'),
            itemQty: itemQty,
            itemAdd: itemAdd,
            itemMinus: itemMinus,
            inputName: inputName,
            inputId: inputId
          };

          // Append new quantity selector then remove original
          el.after(template(data)).remove();
        });

        // Setup listeners to add/subtract from the input
        $('.js--qty-adjuster', this.$container).on('click', function() {
          var el = $(this),
          id = el.data('id'),
          qtySelector = el.siblings('.js--num'),
          qty = parseInt( qtySelector.val() );

          var qty = validateQty(qty);
          qtyMin = timber.cache.$body.hasClass('template-product') ? 1 : qtyMin;

          // Add or subtract from the current quantity
          if (el.hasClass('js--add')) {
            qty = qty + 1;
          } else {
            qty = qty <= qtyMin ? qtyMin : qty - 1;
          }

          // Update the input's number
          qtySelector.val(qty);
        });

      }
    },

    initBreakpoints: function () {

      var self = this;
      var $container = self.$container;
      self.zoomType = $container.data('zoom-enabled');

      enquire.register(timber.vars.mediaQueryLarge, {
        match: function() {
          timber.vars.isLargeBp = true;
          if (self.zoomType) {
            // reinit product zoom
            productImageZoom();
          }

        },
        unmatch: function() {
          timber.vars.isLargeBp = false;
          if (self.zoomType) {

            if ((self.settings.selectors.$productImage).length) {
              // remove event handlers for product zoom on mobile
                self.settings.selectors.$productImage.off();
                self.settings.selectors.$productImageWrapper.trigger('zoom.destroy');
            }
          }

        }
      });
    },

    productImageSwitch: function() {
      if (!this.settings.selectors.$thumbImages.length) {
        return;
      }

      var self = this;

      // Switch the main image with one of the thumbnails
      // Note: this does not change the variant selected, just the image
      self.settings.selectors.$thumbImages.on('click', function(evt) {
        evt.preventDefault();
        var newImageId = $(this).attr('data-image-id');
        self.switchImage(newImageId);
      });
    },

    switchImage: function (imageId) {
      var $newImage = this.settings.selectors.$productImageWrapper.filter('[data-image-id="' + imageId + '"]');
      var $otherImages = this.settings.selectors.$productImageWrapper.not('[data-image-id="' + imageId + '"]');
      $newImage.removeClass('hide');
      $otherImages.addClass('hide');

      if ($newImage.find('img').attr('data-zoom') && timber.vars.isLargeBp) {
        productImageZoom();
      }
    },

    initProductVariant: function() {
      // this.productSingleObject is a global JSON object defined in theme.liquid
      if (!this.productSingleObject) {
        return;
      }

      var self = this;
      this.optionSelector = new Shopify.OptionSelectors(self.settings.selectors.originalSelectorId, {
        selectorClass: self.settings.selectors.$optionSelectorClass,
        product: self.productSingleObject,
        onVariantSelected: self.productVariantCallback.bind(self),
        enableHistoryState: self.settings.enableHistoryState,
        settings: self.settings
      });

      // Clean up variant labels if the Shopify-defined
      // defaults are the only ones left
      this.simplifyVariantLabels(this.productSingleObject);
    },

    simplifyVariantLabels: function(productObject) {
      // Hide variant dropdown if only one exists and title contains 'Default'
      if (productObject.variants.length === 1 && productObject.options.length === 1 && productObject.options[0].toLowerCase().indexOf('title') >= 0 && productObject.variants[0].title.toLowerCase().indexOf('default title') >= 0) {
        $('.selector-wrapper', this.$container).hide();
      }
    },

    productVariantCallback: function(variant) {
      var self = this;

      if (variant) {
        //  Only change unit price for main product
        var $priceContainer = $(this.settings.selectors.priceContainer, this.$container);

        // Update unit price, if one is set
        var $unitPriceContainer = $(this.settings.selectors.unitPriceContainer, $priceContainer);

        $unitPriceContainer.removeClass('product-price-unit--available');

        if (variant.unit_price_measurement) {
          var $unitPrice = $(this.settings.selectors.unitPrice, $priceContainer);
          var $unitPriceBaseUnit = $(this.settings.selectors.unitPriceBaseUnit, $priceContainer);

          $unitPrice.text(Shopify.formatMoney(variant.unit_price, moneyFormat));
          $unitPriceBaseUnit.text(this.getBaseUnit(variant));
          $unitPriceContainer.addClass('product-price-unit--available');
        }

        // Update variant image, if one is set
        if (variant.featured_image) {
          var newImg = variant.featured_image;
          var $newImage = this.settings.selectors.$productImageWrapper.filter('[data-image-id="' + newImg.id + '"]');
          var $otherImages = this.settings.selectors.$productImageWrapper.not('[data-image-id="' + newImg.id + '"]');

          $newImage.removeClass('hide');
          $otherImages.addClass('hide');
        }
        
        // Hide metafields label if variant is out of stock
        if (variant.inventory_quantity < 1) {
          $("#variantQuantityDescription").hide();
        }
        
        if (variant.available) {
          // We have a valid product variant, so enable the submit button
          var qty = variant.inventory_quantity;
          var bodyId = document.body.id;
          this.settings.selectors.$addToCart.removeClass('disabled').prop('disabled', false);
          this.settings.selectors.$addToCartText.html("Add to Cart");
          $(this.settings.selectors.$shopifyPaymentButton, this.$container).show();

          this.settings.selectors.$variantQuantity.removeClass('is-visible');
          this.settings.selectors.$variantIncoming.removeClass('is-visible');

          var $link = this.settings.selectors.$productFullDetails;
          if ($link.length) {
            $link.attr('href', updateUrlParameter($link.attr('href'), 'variant', variant.id));
          }
		
          if (variant.inventory_management) {
            // Show how many items are left, if below 2000
            if (variant.inventory_quantity < 2000 && variant.inventory_quantity > 0 && this.settings.stockSetting == 'true') {
              this.settings.selectors.$variantQuantityMessage.html(theme.strings.product.only_left.replace('1', variant.inventory_quantity));
              this.settings.selectors.$variantQuantity.addClass('is-visible');

             // Trevan - Update variant qty on select
              //jQuery('#the-ttl-store-collect-different-t-shirt .CustomVariantQuantity, #the-ttl-store-dumb-dodo-t-shirt .CustomVariantQuantity , #the-ttl-store-dumpster-fire-t-shirt .CustomVariantQuantity').html(qty + " remaining (of this size) of 250 (all sizes)");
              //jQuery('#monero-invisible-man-shirt .CustomVariantQuantity').html(qty + " remaining (of this size) of 100 (all sizes)");
              //jQuery('#retro-future-tari-shirt-tank .CustomVariantQuantity').html(qty + " remaining (of this size) of 200 (all sizes)");
              //jQuery('#the-ttl-store-tari-gem-hat .CustomVariantQuantity').html(qty + " remaining of 50 (all sizes)"); 
              //jQuery('#the-ttl-store-private-af-hat .CustomVariantQuantity').html(qty + " remaining of 50 (all sizes)"); 
              //jQuery('#the-ttl-store-kid-s-collect-different-shirt .CustomVariantQuantity').html(qty + " remaining of 50 (all sizes)"); 
              //jQuery('#the-ttl-store-tari-protocol-shirt .CustomVariantQuantity').html(qty + " remaining (of this size) of 72 (all sizes)"); 
              //jQuery('#the-ttl-store-tari-privacy-shirt .CustomVariantQuantity').html(qty + " remaining (of this size) of 72 (all sizes)"); 
              jQuery(`#${bodyId} .CustomVariantQuantity`).html(qty); 
          	  $("#variantQuantityDescription").show();
              
            }
          }

          // Show next ship date if quantity <= 0 and stock is incoming
          if (variant.inventory_quantity <= 0 && variant.incoming != null ) {
            if (variant.next_incoming_date != null){
              this.settings.selectors.$variantIncomingMessage.html(theme.strings.product.will_be_in_stock_after.replace('[date]', variant.next_incoming_date));
              this.settings.selectors.$variantIncoming.addClass('is-visible')
            }
          }
        } else {

          // Variant is sold out, disable the submit button
          this.settings.selectors.$addToCart.addClass('disabled').prop('disabled', true);
          jQuery('#the-ttl-store-collect-different-t-shirt .CustomVariantQuantity, #the-ttl-store-dumb-dodo-t-shirt .CustomVariantQuantity , #the-ttl-store-dumpster-fire-t-shirt .CustomVariantQuantity').html("This size is no longer available");
          this.settings.selectors.$addToCartText.html("Sold Out");
          $(this.settings.selectors.$shopifyPaymentButton, this.$container).hide();

          this.settings.selectors.$variantQuantity.removeClass('is-visible');
          this.settings.selectors.$variantIncoming.removeClass('is-visible');
          

          // Show next stock incoming date if stock is incoming
          if (variant.inventory_management) {
            if (variant.incoming && this.settings.incomingMessage == 'true' && variant.incoming != null && variant.next_incoming_date != null) {
              this.settings.selectors.$variantIncoming.html(theme.strings.product.will_be_in_stock_after.replace('[date]', variant.next_incoming_date)).addClass('is-visible');
            }
          }

          this.settings.selectors.$quantityElements.hide();
        }

        // Regardless of stock, update the product price
        var customPrice = timber.formatMoney( Shopify.formatMoney(variant.price, moneyFormat) );
        var a11yPrice = Shopify.formatMoney(variant.price, moneyFormat);
        var customPriceFormat = ' <span aria-hidden="true">' + customPrice + '</span>';
        customPriceFormat += ' <span class="visually-hidden">' + a11yPrice + '</span>';

        // Show SKU
        this.settings.selectors.$SKU.html(variant.sku)

        if (this.settings.showComparePrice == 'true' ) {
          if (variant.compare_at_price > variant.price) {
            var comparePrice = timber.formatMoney(Shopify.formatMoney(variant.compare_at_price, moneyFormat));
            var a11yComparePrice = Shopify.formatMoney(variant.compare_at_price, moneyFormat);

            customPriceFormat = ' <span aria-hidden="true">' + customPrice + '</span>';
            customPriceFormat += ' <span aria-hidden="true"><small><s>' + comparePrice + '</s></small></span>';
            customPriceFormat += ' <span class="visually-hidden"><span class="visually-hidden">Regular price</span> ' + a11yComparePrice + '</span>';
            customPriceFormat += ' <span class="visually-hidden"><span class="visually-hidden">Sale price</span> ' + a11yPrice + '</span>';
          }
        }
        // this.settings.selectors.$productPrice.html(customPriceFormat); CHANGED

        // Also update and show the product's compare price if necessary
        if ( variant.compare_at_price > variant.price ) {
          var priceSaving = timber.formatSaleTag( Shopify.formatMoney(variant.compare_at_price - variant.price, moneyFormat) );
          // priceSaving += ' (' + ( (variant.compare_at_price - variant.price)*100/(variant.compare_at_price) ).toFixed(0) + '%)';
          this.settings.selectors.$comparePrice.html("Save [$]".replace('[$]', priceSaving)).show();
        } else {
          this.settings.selectors.$comparePrice.hide();
        }

      } else {
        // The variant doesn't exist, disable submit button.
        // This may be an error or notice that a specific variant is not available.
        this.settings.selectors.$addToCart.addClass('disabled').prop('disabled', true);
        this.settings.selectors.$addToCartText.html(theme.strings.product.unavailable);
        this.settings.selectors.$variantQuantity.removeClass('is-visible');
        this.settings.selectors.$quantityElements.hide();
        $(this.settings.selectors.$shopifyPaymentButton, this.$container).hide();
      }
    },

    getBaseUnit: function (variant) {
      return variant.unit_price_measurement.reference_value === 1
        ? variant.unit_price_measurement.reference_unit
        : variant.unit_price_measurement.reference_value +
            variant.unit_price_measurement.reference_unit;
    }
  });

  function updateUrlParameter(url, key, value) {
    var re = new RegExp('([?&])' + key + '=.*?(&|$)', 'i');
    var separator = url.indexOf('?') === -1 ? '?' : '&';

    if (url.match(re)) {
      return url.replace(re, '$1' + key + '=' + value + '$2');
    } else {
      return url + separator + key + '=' + value;
    }
  }

  function productImageZoom() {
    var $productImageWrapper = $('.product__image-wrapper');

    if (timber.vars.isLargeBp) {
      if (!$productImageWrapper.length || timber.cache.$html.hasClass('supports-touch')) {
        return;
      };

      // Destroy zoom (in case it was already set), then set it up again
      $productImageWrapper.trigger('zoom.destroy');
      $productImageWrapper.each(function() {
        if($(this).find('img').attr('data-zoom')){
          $(this).addClass('image-zoom').zoom({
            url: $(this).find('img').attr('data-zoom'),
            onZoomIn: function() {
                $(this).prev('img').hide();
            },
            onZoomOut: function() {
                $(this).css('opacity', '0');
                $(this).prev('img').show();
            }
          })
        }
      });
    }
  }

  return Product;

})();

window.theme = window.theme || {};

theme.Search = (function() {
  function Search(container) {
    var $container = (this.$container = $(container));
    timber.cacheSelectors();
    timber.resizeElements($container);

    $(window).resize(function() {
      timber.resizeElements($container);
    });
  }

  return Search;
})();

theme.Slideshow = function(el) {
  this.cache = {
    $slider: $(el),
    sliderArgs: {
      animation: 'slide',
      animationSpeed: 500,
      pauseOnHover: true,
      keyboard: false,
      slideshow: $(el).data('slider-home-auto'),
      slideshowSpeed: $(el).data('slider-home-rate'),
      smoothHeight: true,
      before: function(slider) {
        $(slider).resize();
        $(slider)
          .find('.slide')
          .not('.flex-active-slide')
          .removeClass('slide-hide');
      },
      after: function(slider) {
        $(slider)
          .find('.slide')
          .not('.flex-active-slide')
          .addClass('slide-hide');
        $(slider).resize();
      },
      start: function(slider) {
        $(slider)
          .find('.slide')
          .not('.flex-active-slide')
          .addClass('slide-hide');
        if (
          $(slider)
            .find('.slide')
            .not('.clone').length === 1
        ) {
          $(slider)
            .find('.flex-direction-nav')
            .remove();
        }
        $(window).trigger('resize');
        slider.addClass('loaded');
        if ($('#slider').data('loaded-index') !== undefined) {
          $('#slider').flexslider($('#slider').data('loaded-index'));
        }
      }
    }
  };
  if (this.cache.$slider.find('li').length === 1) {
    this.cache.sliderArgs.touch = false;
  }
  this.cache.$slider.flexslider(this.cache.sliderArgs);
};

theme.slideshows = theme.slideshows || {};

theme.SlideshowSection = (function() {
  function SlideshowSection(container) {
    var $container = (this.$container = $(container));
    var id = $container.attr('data-section-id');
    var slideshow = (this.slideshow = '#heroSlider--' + id);
    var numberOfSlides = $(slideshow).find('li').length;

    if (numberOfSlides <= 0) return;

    theme.slideshows[slideshow] = new theme.Slideshow(slideshow);
  }

  return SlideshowSection;
})();

theme.SlideshowSection.prototype = _.assignIn(
  {},
  theme.SlideshowSection.prototype,
  {
    onUnload: function() {
      delete theme.slideshows[this.slideshow];
    },

    onBlockSelect: function(evt) {
      var $slideshow = $(this.slideshow);
      var $slide = $('#slide--' + evt.detail.blockId + ':not(.clone)');

      var slideIndex = $slide.data('flexslider-index');
      var $slideImg = $slide.find('img') || $slide.find('svg');

      $slide.imagesLoaded($slideImg, function() {
        $slideshow.flexslider(slideIndex);
        $slideshow.resize();
        $slideshow.flexslider('pause');
      });
    },

    onBlockDeselect: function() {
      $(this.slideshow).flexslider('play');
    }
  }
);


$(document).ready(function() {
  var sections = new theme.Sections();
  sections.register('collections-list-template', theme.FeaturedCollections);
  sections.register('collection-row-section', theme.CollectionRows);
  sections.register('collection-template', theme.Collection);
  sections.register('header-section', theme.HeaderSection);
  sections.register('list-collections-template', theme.ListCollections);
  sections.register('map-section', theme.Maps);
  sections.register('product-template', theme.Product);
  sections.register('search-template', theme.Search);
  sections.register('slideshow-section', theme.SlideshowSection);
});
