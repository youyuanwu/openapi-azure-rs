Run these commands from the repository root. Both crates use OpenAPI Generator
**7.26.0**. The Service Fabric script pins the generator JAR and its SHA-256;
the Blob Docker image is also pinned so regeneration is reproducible.

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

bash generation/service-fabric/generate.sh
```

Service Fabric regeneration requires a JDK (Java 11 or later), `curl`,
`sha256sum`, and Cargo/Rustfmt. The script caches the JAR outside the repository,
compiles `ServiceFabricCodegen.java` in a temporary directory, and generates only
models/APIs, preserving the checked-in manifest, module indexes, and README.
It reads `specs/servicefabric.json` without modifying it.

The subclass customizes generator metadata rather than editing the specification:

- Use each subtype's `x-ms-discriminator-value` and all inherited/derived fields
  in tagged enums, excluding the discriminator field emitted by Serde.
- Model the omitted `upload_file` and `upload_file_chunk` bodies as required raw
  `Vec<u8>` arguments, including intentionally empty directory markers.
- Represent query parameters ending in `HealthStateFilter` as `u32` masks
  (`0..=4294967295`), serialized as positive decimal values.
- Mark model-returning operations with documented bodyless 2xx alternatives as
  `Option<T>`; only those exact statuses become `None`, not decoding failures.
  These include application lookup's 204 and Mesh create/update's 202.

`generation/service-fabric/templates/reqwest/api.mustache` was scaffolded from
the pinned generator's `rust/reqwest/api.mustache`. Its focused overrides consume
the above metadata, require 2xx success, and return `reqwest::StatusCode` instead
of `()` for bodyless operations. Keep these overrides when updating the generator.
The stock `supportMultipleResponses` option was tested but is not enabled:
its untagged success decoding suppresses malformed-body errors and does not
select response variants by HTTP status.

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
    --additional-properties=packageName=openapi-azure-storage-blob,packageVersion=0.1.2,useChrono=false


sudo chown -R $(whoami) .

sed -i 's/license = "Unlicense"/license = "MIT"/' openapi-azure-storage-blob/Cargo.toml
sed -i '/edition = "2021"/a\documentation = "https://learn.microsoft.com/en-us/azure/storage/blobs/"' openapi-azure-storage-blob/Cargo.toml
sed -i '/edition = "2021"/a\readme = "README.md"' openapi-azure-storage-blob/Cargo.toml
sed -i '/edition = "2021"/a\repository.workspace = true' openapi-azure-storage-blob/Cargo.toml
sed -i '/^rustls = /a\rustls-tls = ["rustls"]' openapi-azure-storage-blob/Cargo.toml
sed -i '/unused_imports)]/i\#![allow(clippy::needless_return,clippy::into_iter_on_ref)]' openapi-azure-storage-blob/src/lib.rs
cargo fmt
```

The generator now uses Reqwest 0.13. Set `useChrono=false` to retain the existing
string-based date/time API: callers supply ISO8601 backup query filters and
RFC7231 conditional Blob headers. The default Chrono generation serializes
these request values with `Display`, which does not match their wire formats.
The `rustls-tls` feature alias preserves the previous feature name alongside the
generated `rustls` feature. Blob file bodies are now used by the generated
upload code, so the old unused-body rename must not be applied.

Blob generation resets dependency requirements in its generated manifest.
Reapply the current dependency minimums with Cargo before validating
(the Service Fabric script preserves its manifest):
```bash
cargo add -p openapi-azure-storage-blob serde@1.0.229 serde_with@3.24.0 serde_json@1.0.154 serde_repr@0.1.21 url@2.5.8 uuid@1.28.0 tokio@1.54.1 tokio-util@0.7.20 reqwest@0.13.5
cargo update
```
These are the latest stable direct dependency versions at update time.
`cargo update` also refreshes transitive dependencies to their newest compatible
versions. Keep the workspace Tokio requirement in sync with the Blob crate.

Validate after regenerating and applying the post-processing:
```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets -j 1
cargo test -p openapi-service-fabric --no-default-features --test interoperability -j 1
cargo test -p openapi-service-fabric -p openapi-azure-storage-blob -j 1
cargo clippy --workspace --all-targets -j 1
cargo check -p openapi-service-fabric -p openapi-azure-storage-blob --no-default-features --features rustls-tls -j 1
```
Clippy reports existing generated manual `Default` implementations as
`derivable_impls` warnings and large error variants as `result_large_err` warnings.

The `openapi-tests` integration test requires a local Service Fabric onebox
cluster at `http://localhost:19080`; run it only when that cluster is available.

See openapi codegen [rust docs](https://openapi-generator.tech/docs/generators/rust/)