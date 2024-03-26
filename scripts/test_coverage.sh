#!/bin/bash
# Prerequisites
# 1. You need LLVM-COV tools:
# $ rustup component add llvm-tools-preview
# 2. and Rust wrappers for llvm-cov:
# $ cargo install cargo-binutils
# 3. The rust name demangler
# $ cargo install rustfilt
# 4. jq
# 5. genhtml
# $ sudo apt install lcov

RUSTFLAGS="-C instrument-coverage"
RUSTUP_TOOLCHAIN=${RUSTUP_TOOLCHAIN:-nightly}
PACKAGE=tari_jwt
echo "Using ${RUSTUP_TOOLCHAIN} toolchain"
LLVM_PROFILE_FILE="./cov_raw/${PACKAGE}-%m.profraw"

get_binaries() {
  files=$( RUSTFLAGS=$RUSTFLAGS cargo +${RUSTUP_TOOLCHAIN} test --tests --no-run --message-format=json \
              | jq -r "select(.profile.test == true) | .filenames[]" \
              | grep -v dSYM - \
        );
  files=("${files[@]/#/-object }")
}

get_binaries

echo "** Generating ..."
echo ${files}
# Remove old coverage files
rm -fr cov_raw coverage_report default*.profraw

RUSTFLAGS=$RUSTFLAGS LLVM_PROFILE_FILE=${LLVM_PROFILE_FILE} cargo +${RUSTUP_TOOLCHAIN} test --tests

cargo profdata -- \
  merge -sparse ./cov_raw/${PACKAGE}-*.profraw -o ./cov_raw/${PACKAGE}.profdata

cargo cov -- \
  export \
    --Xdemangler=rustfilt \
    --format=lcov \
    --show-branch-summary \
    --show-instantiation-summary \
    --show-region-summary \
    --ignore-filename-regex='\.cargo' \
    --ignore-filename-regex="rustc" \
    --ignore-filename-regex="\.git" \
    --instr-profile=cov_raw/${PACKAGE}.profdata \
    $files \
    > cov_raw/${PACKAGE}.lcov

cargo cov -- \
  show \
    --Xdemangler=rustfilt \
    --show-branch-summary \
    --show-instantiation-summary \
    --show-region-summary \
    --ignore-filename-regex='\.cargo' \
    --ignore-filename-regex="rustc" \
    --ignore-filename-regex="\.git" \
    --instr-profile=cov_raw/${PACKAGE}.profdata \
    $files \
    > cov_raw/${PACKAGE}.txt

if [ -z ${SKIP_HTML+x} ]; then
  genhtml -o coverage_report cov_raw/${PACKAGE}.lcov
else
  echo "Skipping html generation"
fi
# open coverage_report/src/index.html
