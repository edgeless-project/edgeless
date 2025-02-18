#!/bin/bash

# ===========================================================
# Colors for logging
CLR_RED='\033[0;31m'
CLR_GREEN='\033[0;32m'
CLR_YELLOW='\033[0;33m'
CLR_RST='\033[0m'

# DEBUG prints
function echo_r(){ echo -e "${CLR_RED}$*${CLR_RST}"; }
function echo_g(){ echo -e "${CLR_GREEN}$*${CLR_RST}"; }
function echo_y(){ echo -e "${CLR_YELLOW}$*${CLR_RST}"; }
# ===========================================================

# edgeless_cli path is deduced from where the script is located
if [ -x "$(dirname "$0")/../target/debug/edgeless_cli" ]; then
    CLI="$(dirname "$0")/../target/debug/edgeless_cli"
elif [ -x "$(dirname "$0")/../target/release/edgeless_cli" ]; then
    CLI="$(dirname "$0")/../target/release/edgeless_cli"
else
    echo_r "edgeless_cli not found in 'target/debug' or 'target/release'. Please build the edgeless binaries before running this script."
    exit 1
fi

rm -f build_functions.log 2> /dev/null
RC=0

for func in $(find $(dirname "$0")/../functions -type f -name function.json) ; do
    name=$(basename $(dirname $func))
    echo -n "$name: "

    wasm_name="$(dirname $func)/$(grep "\"id\"" $func | cut -f 4 -d '"').wasm"
    if [ -r $wasm_name ] ; then
        echo_g "[OK]"
    else
        echo -n "building."
        ${CLI} function build $func >> build_functions.log 2>&1
        if [ "$?" -ne 0 ] ; then
            echo_r ".[FAILED]"
            RC=$(( RC + 1 ))
        else
            echo_g ".[OK]"
        fi
    fi
done

exit ${RC}
