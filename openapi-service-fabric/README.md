# openapi-service-fabric
Auto generated service fabric http client for Rust.

## REST interoperability

Tagged enums use the documented wire discriminator values (for example,
`Stateful`, `Event`, and `ImageStorePath`) and preserve inherited and subtype
fields. Provision requests must include the subtype's required fields.

Model-returning operations with documented bodyless success alternatives return
`Option<T>`: `Some` for decoded responses and `None` only for those exact statuses.
This includes `get_application_info`'s 204 and Mesh create/update's 202.
Malformed 200/201 responses remain errors.

Operations previously returning `()` now return `reqwest::StatusCode`, so callers
can distinguish completion from acceptance or enforce an exact status contract.
Only 2xx responses are successful. Disable redirects on the configured Reqwest
client when the original response status must be observed; non-2xx responses
are exposed through `Error::ResponseError`, including their status and body.

`upload_file` and `upload_file_chunk` accept a final `body: Vec<u8>` argument,
sent unchanged as `application/octet-stream`. Pass `Vec::new()` for an empty
`_.dir` marker. Health-state-filter query arguments use `Option<u32>` and accept
the full unsigned 32-bit range without negative-value reinterpretation.

These changes affect source compatibility with 0.1.1. Regenerate with
`bash generation/service-fabric/generate.sh` from the repository root; see
[`docs/Dev.md`](../docs/Dev.md) for the generator extension and validation steps.
The OpenAPI specification is unchanged.