Download the swagger file:
```bash
wget https://raw.githubusercontent.com/Azure/azure-rest-api-specs/refs/heads/main/specification/servicefabric/data-plane/ServiceFabric/stable/8.2/servicefabric.json \
    -O ./specs/servicefabric.json

docker run --rm -v "${PWD}:/local" openapitools/openapi-generator-cli generate \
    -i /local/specs/servicefabric.json \
    -g rust\
    -o /local/openapi-service-fabric/ \
    --global-property=apiDocs=false,modelDocs=false \
    --additional-properties=packageName=openapi-service-fabric,packageVersion=0.1.0

sudo chown -R $(whoami) .
cargo fmt

sed -i 's/license = "Unlicense"/license = "MIT"/' openapi-service-fabric/Cargo.toml
sed -i '/edition = "2021"/a\documentation = "https://learn.microsoft.com/en-us/azure/service-fabric/service-fabric-overview"' openapi-service-fabric/Cargo.toml
sed -i '/edition = "2021"/a\readme = "README.md"' openapi-service-fabric/Cargo.toml
sed -i '/edition = "2021"/a\repository.workspace = true' openapi-service-fabric/Cargo.toml
sed -i '/unused_imports)]/i\#![allow(clippy::needless_return)]' openapi-service-fabric/src/lib.rs
```

Azure storage
```bash
wget https://raw.githubusercontent.com/Azure/azure-rest-api-specs/refs/heads/main/specification/storage/data-plane/Microsoft.BlobStorage/stable/2026-02-06/generated_blob.json \
    -O ./specs/generated_blob.json 

docker run --rm -v "${PWD}:/local" openapitools/openapi-generator-cli generate \
    -i /local/specs/generated_blob.json \
    -g rust \
    -o /local/openapi-azure-storage-blob/ \
    --global-property=apiDocs=false,modelDocs=false \
    --additional-properties=packageName=openapi-azure-storage-blob,packageVersion=0.1.1


sudo chown -R $(whoami) .
cargo fmt

sed -i 's/license = "Unlicense"/license = "MIT"/' openapi-azure-storage-blob/Cargo.toml
sed -i '/edition = "2021"/a\documentation = "https://learn.microsoft.com/en-us/azure/storage/blobs/"' openapi-azure-storage-blob/Cargo.toml
sed -i '/edition = "2021"/a\readme = "README.md"' openapi-azure-storage-blob/Cargo.toml
sed -i '/edition = "2021"/a\repository.workspace = true' openapi-azure-storage-blob/Cargo.toml
sed -i '/unused_imports)]/i\#![allow(clippy::needless_return,clippy::into_iter_on_ref)]' openapi-azure-storage-blob/src/lib.rs
# unsupported body
sed -i 's/let p_form_body/let _p_form_body/' openapi-azure-storage-blob/src/apis/default_api.rs
```
See openapi codegen [rust docs](https://openapi-generator.tech/docs/generators/rust/)