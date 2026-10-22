#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
generator_version=7.26.0
generator_sha256=1760050094997b9cc790cc1350be095f15da8c08a996b184d3eadc4ccda5e35e
cache="${XDG_CACHE_HOME:-$HOME/.cache}/openapi-generator"
jar="$cache/openapi-generator-cli-$generator_version.jar"
mkdir -p "$cache"
if [[ ! -f "$jar" ]]; then
    curl --fail --location --output "$jar" \
        "https://repo.maven.apache.org/maven2/org/openapitools/openapi-generator-cli/$generator_version/openapi-generator-cli-$generator_version.jar"
fi
echo "$generator_sha256  $jar" | sha256sum --check

build="$(mktemp -d)"
trap 'rm -rf "$build"' EXIT
javac -cp "$jar" -d "$build" "$root/generation/service-fabric/ServiceFabricCodegen.java"
java -Xmx768m -cp "$build:$jar" org.openapitools.codegen.OpenAPIGenerator generate \
    -g ServiceFabricCodegen \
    -i "$root/specs/servicefabric.json" \
    -t "$root/generation/service-fabric/templates" \
    -o "$root/openapi-service-fabric" \
    --global-property=apis,models,apiDocs=false,modelDocs=false \
    --additional-properties=packageName=openapi-service-fabric,packageVersion=0.1.1,useChrono=false

cd "$root"
cargo fmt -p openapi-service-fabric
