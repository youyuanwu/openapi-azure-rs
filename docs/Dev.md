Download the swagger file:
```bash
mkdir build
wget https://raw.githubusercontent.com/Azure/azure-rest-api-specs/refs/heads/main/specification/servicefabric/data-plane/ServiceFabric/stable/8.2/servicefabric.json -O ./build/servicefabric.json

docker run --rm -v "${PWD}:/local" openapitools/openapi-generator-cli generate \
    -i /local/build/servicefabric.json \
    -g rust\
    -o /local/mssf-http/ \
    --global-property=apiDocs=false,modelDocs=false \
    --additional-properties=packageName=mssf-http,packageVersion=0.1.0

sudo chown -R $(whoami) .
cargo fmt
```
See openapi codegen [rust docs](https://openapi-generator.tech/docs/generators/rust/)