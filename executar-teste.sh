#!/usr/bin/bash

# Use este script para executar testes locais

RESULTS_WORKSPACE="$HOME/gatling/3.10.3/load-test/user-files/results"
GATLING_BIN_DIR=$HOME/gatling/3.10.3/bin
GATLING_WORKSPACE="$HOME/gatling/3.10.3/user-files"

runGatling() {
    sh $GATLING_BIN_DIR/gatling.sh -rf $RESULTS_WORKSPACE \
        -sf "$GATLING_WORKSPACE/simulations"
}

startTest() {
    for i in {1..20}; do
        # 2 requests to wake the 2 api instances up :)
        curl --fail http://localhost:9999/clientes/1/extrato && \
        echo "" && \
        curl --fail http://localhost:9999/clientes/1/extrato && \
        echo "" && \
        runGatling && \
        break || sleep 2;
    done
}

startTest
