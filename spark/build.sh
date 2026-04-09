#!/bin/bash
export PATH="/usr/libexec/spark/bin:$PATH"
cd "$(dirname "$0")"
gprbuild -P spark.gpr "$@"
