Run these commands from the repository root. Both crates were regenerated with
OpenAPI Generator **7.26.0**, the latest stable release at regeneration time.
The Docker image is pinned so regeneration is reproducible.

Rust and Cargo are pinned to **1.99.1**, the latest stable toolchain at update
time, in `rust-toolchain.toml`. Rustup installs it with the Rustfmt and Clippy
components automatically when running Cargo in this repository.

When regenerating, use the existing files in `specs/` unchanged. Only run the
optional `wget` commands when intentionally refreshing the specifications.

Service Fabric:
```bash
# Optional: refresh the specification (skip for regeneration).
wget https://raw.githubusercontent.com/Azure/azure-rest-api-specs/refs/heads/main/specification/servicefabric/data-plane/ServiceFabric/stable/8.2/servicefabric.json \
    -O ./specs/servicefabric.json

docker run --rm -v "${PWD}:/local" openapitools/openapi-generator-cli:v7.26.0 generate \
    -i /local/specs/servicefabric.json \
    -g rust\
    -o /local/openapi-service-fabric/ \
    --global-property=apiDocs=false,modelDocs=false \
    --additional-properties=packageName=openapi-service-fabric,packageVersion=0.1.0

sudo chown -R $(whoami) .

sed -i 's/license = "Unlicense"/license = "MIT"/' openapi-service-fabric/Cargo.toml
sed -i '/edition = "2021"/a\documentation = "https://learn.microsoft.com/en-us/azure/service-fabric/service-fabric-overview"' openapi-service-fabric/Cargo.toml
sed -i '/edition = "2021"/a\readme = "README.md"' openapi-service-fabric/Cargo.toml
sed -i '/edition = "2021"/a\repository.workspace = true' openapi-service-fabric/Cargo.toml
sed -i '/^rustls = /a\rustls-tls = ["rustls"]' openapi-service-fabric/Cargo.toml
sed -i '/unused_imports)]/i\#![allow(clippy::needless_return)]' openapi-service-fabric/src/lib.rs
cargo fmt
```

Azure storage
```bash
# Optional: refresh the specification (skip for regeneration).
wget https://raw.githubusercontent.com/Azure/azure-rest-api-specs/refs/heads/main/specification/storage/data-plane/Microsoft.BlobStorage/stable/2026-02-06/generated_blob.json \
    -O ./specs/generated_blob.json 

docker run --rm -v "${PWD}:/local" openapitools/openapi-generator-cli:v7.26.0 generate \
    -i /local/specs/generated_blob.json \
    -g rust \
    -o /local/openapi-azure-storage-blob/ \
    --global-property=apiDocs=false,modelDocs=false \
    --additional-properties=packageName=openapi-azure-storage-blob,packageVersion=0.1.1


sudo chown -R $(whoami) .

sed -i 's/license = "Unlicense"/license = "MIT"/' openapi-azure-storage-blob/Cargo.toml
sed -i '/edition = "2021"/a\documentation = "https://learn.microsoft.com/en-us/azure/storage/blobs/"' openapi-azure-storage-blob/Cargo.toml
sed -i '/edition = "2021"/a\readme = "README.md"' openapi-azure-storage-blob/Cargo.toml
sed -i '/edition = "2021"/a\repository.workspace = true' openapi-azure-storage-blob/Cargo.toml
sed -i '/^rustls = /a\rustls-tls = ["rustls"]' openapi-azure-storage-blob/Cargo.toml
sed -i '/unused_imports)]/i\#![allow(clippy::needless_return,clippy::into_iter_on_ref)]' openapi-azure-storage-blob/src/lib.rs
cargo fmt
```

The generator now uses Reqwest 0.13 and Chrono date/time types. The `rustls-tls`
feature alias preserves the previous feature name alongside the generated
`rustls` feature. Blob file bodies are now used by the generated upload code, so
the old unused-body rename must not be applied.

Generation resets dependency requirements in the generated manifests. Reapply
the current dependency minimums with Cargo before validating:
```bash
cargo add -p openapi-service-fabric serde@1.0.229 serde_json@1.0.154 serde_repr@0.1.21 url@2.5.8 uuid@1.28.0 chrono@0.4.45 reqwest@0.13.5
cargo add -p openapi-azure-storage-blob serde@1.0.229 serde_with@3.24.0 serde_json@1.0.154 serde_repr@0.1.21 url@2.5.8 uuid@1.28.0 chrono@0.4.45 tokio@1.54.1 tokio-util@0.7.20 reqwest@0.13.5
cargo update
```
These are the latest stable direct dependency versions at update time.
`cargo update` also refreshes transitive dependencies to their newest compatible
versions. Keep the workspace Tokio requirement in sync with the Blob crate.

Validate after regenerating and applying the post-processing:
```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets -j 1
cargo test -p openapi-service-fabric -p openapi-azure-storage-blob -j 1
cargo clippy --workspace --all-targets -j 1
cargo check -p openapi-service-fabric -p openapi-azure-storage-blob --no-default-features --features rustls-tls -j 1
```
Clippy reports existing generated manual `Default` implementations as
`derivable_impls` warnings.

The `openapi-tests` integration test requires a local Service Fabric onebox
cluster at `http://localhost:19080`; run it only when that cluster is available.

See openapi codegen [rust docs](https://openapi-generator.tech/docs/generators/rust/)