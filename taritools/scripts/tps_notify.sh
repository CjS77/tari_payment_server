#!/bin/bash
# Notify script for Tari Payment Server Hot wallet.

# Install this script in `$HOME/.taritools/tps_notify.sh` and make it executable.

# post to a webhook url
BINPATH=${TARITOOLS_PATH:-$HOME/.cargo/bin}
BIN=$BINPATH/taritools
PROFILE="TPS Hot Wallet"

register_received_payment() {
  ${BIN} wallet received --profile "$PROFILE" --amount $2 --txid $3 --message "$4" --sender $5
}

register_confirmation() {
  ${BIN} wallet confirmation --profile "$PROFILE" --txid $3
}

# Log the event
echo "$@" >> $HOME/.taritools/tps_notify.log

if [ -z "${12}" ]; then
  if [ "$1" == "received" ]; then
    register_received_payment "$@"
  elif [ "$1" == "confirmed" ]; then
    register_confirmation "$@"
  else
    echo "Unhandled main event: $@"
  fi
else
  echo "Unhandled short event: $@"
# TODO - handle cancellations
fi

# Argument key:
# For transaction received, mined(unconfirmed), and mined events:
#  $1 = "received", "confirmation", or "mined"
#  $2 = amount,
#  $3 = tx_id
#  $4 = message
#  $5 = source address public key
#  $6 = destination address public key
#  $7 = status
#  $8 = excess,
#  $9 = public_nonce,
# $10 = signature,
# $11 = number of confirmations (if applicable, otherwise empty string)
# $12 = direction

# 2.
# For transaction "sent" event, we only have the pending outbound transaction:
# $1 = "sent"
# $2 = amount,
# $3 = tx_id
# $4 = message
# $5 = destination address public key
# $6 = status,
# $7 = direction,

# 3.
# For a transaction "cancelled" event, if it was still pending - it would have the same args as 2. (with $5 as source address public key if inbound).
# If the cancelled tx was already out of pending state, the cancelled event will have the same args as 1.
