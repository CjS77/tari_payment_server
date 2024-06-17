#!/bin/bash

check_for() {
  echo -n "Checking for $1... "
  if ! command -v $2 &> /dev/null
  then
      echo "$1 could not be found. Installing"
      $3
  else
      echo "ok!"
  fi
}

check_for "profdata" "cargo profdata -V" "rustup component add llvm-tools-preview"
check_for "llvm-cov" "cargo llvm-cov -V" "rustup component add llvm-tools-preview"

export RUST_LOG="error,tari_payment_server=trace,tari_payment_engine=trace,tpg_common=trace,e2e_tests=trace,sqlx=warn"
export DATABASE_URL=sqlite://data/tari_store.db
export DATABASE_TYPE=sqlite

cargo llvm-cov clean --workspace # remove artifacts that may affect the coverage results

GEN_HTML=1
# If LLVM_LCOV envar is set, set GEN_HTML to 0
[ -n "$LLVM_LCOV" ] && GEN_HTML=0

if [ $GEN_HTML -eq 1 ]; then
  echo "Generating HTML coverage report"
  cargo llvm-cov --workspace --features test_utils --ignore-filename-regex taritools --html
  open target/llvm-cov/html/index.html
else
  echo "Generating LCOV coverage report"
  cargo llvm-cov --workspace --lcov --output-path lcov.info --ignore-filename-regex taritools --features test_utils
fi


