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
 
if [ "$CLI" == "" ] ; then
    CLI=target/debug/edgeless_cli
fi
if [ ! -x "$CLI" ] ; then
    echo_r "cli not found in '$CLI', specify its path in the CLI environment variable"
    exit 1
fi

rm -f build_functions.log 2> /dev/null
RET=0
for func in $(find functions -type f -name function.json) ; do
    name=$(basename $(dirname $func))
    echo -n "$name: "
    
    wasm_name="$(dirname $func)/$(grep "\"id\"" $func | cut -f 4 -d '"').wasm"
    if [ -r $wasm_name ] ; then
        echo_g "[OK]"
    else
        echo -n "building."
        target/debug/edgeless_cli function build $func >> build_functions.log 2>&1
        if [ "$?" -ne 0 ] ; then
            echo_r ".[FAILED]"
            RET=$(( RET + 1 ))
        else
            echo_g ".[OK]"
        fi
    fi
done
exit $RET