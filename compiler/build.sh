#!/bin/bash

set -euv -o pipefail

cd "base"

perform_push="${PERFORM_PUSH-false}"

docker build -t "javaplayground/valhalla"\
    --platform linux/amd64\
    --build-arg TARGZ_URL=https://download.java.net/java/early_access/valhalla/1/openjdk-23-valhalla+1-90_linux-x64_bin.tar.gz\
    --build-arg TARGZ_SHA=5235afaf5ecc86f2237458cf40f8ed965939372f606edbd0fc46e1ee2e69f5f5\
    --build-arg TARGZ_FOLDER=jdk-23\
    .

docker build -t "javaplayground/latest"\
    --platform linux/amd64\
    --build-arg TARGZ_URL=https://download.java.net/java/GA/jdk22.0.2/c9ecb94cd31b495da20a27d4581645e8/9/GPL/openjdk-22.0.2_linux-x64_bin.tar.gz\
    --build-arg TARGZ_SHA=41536f115668308ecf4eba92aaf6acaeb0936225828b741efd83b6173ba82963\
    --build-arg TARGZ_FOLDER=jdk-22.0.2\
    .

docker build -t "javaplayground/early_access"\
    --platform linux/amd64\
    --build-arg TARGZ_URL=https://download.java.net/java/GA/jdk23.0.1/c28985cbf10d4e648e4004050f8781aa/11/GPL/openjdk-23.0.1_linux-x64_bin.tar.gz\
    --build-arg TARGZ_SHA=dc9b6adc1550afd95e30e131c1c38044925cb656923f92f6dbf0fbd8c1405629\
    --build-arg TARGZ_FOLDER=jdk-23.0.1\
    .


if [[ "${perform_push}" == 'true' ]]; then
    docker push "javaplayground/valhalla"
    docker push "javaplayground/latest"
    docker push "javaplayground/early_access"
fi