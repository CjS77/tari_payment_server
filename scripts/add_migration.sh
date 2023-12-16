#!/bin/bash

# This script is used to create a new migration file
# It uses the DATABASE_TYPE environment variable to determine which database to use

source .env

if [[ -z "$DATABASE_TYPE" ]]; then
  echo "DATABASE_TYPE environment variable is not set"
  exit 1
fi

sqlx migrate add -r -s --source ./src/db/$DATABASE_TYPE/migrations $@
