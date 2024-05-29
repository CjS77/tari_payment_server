#!/bin/bash

# This script is used to create a new migration file
# It uses the DATABASE_TYPE environment variable to determine which database to use

source .env

if [[ -z "$DATABASE_TYPE" ]]; then
  echo "DATABASE_TYPE environment variable is not set"
  exit 1
fi

echo "Running migrations for $DATABASE_TYPE"
# If DATABASE_TYPE is 'sqlite', then check if $DATABASE_URL exists
if [[ "$DATABASE_TYPE" == "sqlite" ]]; then
  DATABASE_FILE="${DATABASE_URL/sqlite:\/\/}"
  if [[ ! -f "$DATABASE_FILE" ]]; then
    echo "Creating database: $DATABASE_FILE"
    touch $DATABASE_FILE
  fi
fi


CMD=${1:-run}
shift;

sqlx migrate $CMD --source ./tari_payment_engine/src/$DATABASE_TYPE/migrations $@
echo "Ok"
