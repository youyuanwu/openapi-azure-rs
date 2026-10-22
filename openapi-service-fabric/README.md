# openapi-service-fabric
Auto generated service fabric http client for Rust.

## REST interoperability

Tagged enums use the documented wire discriminator values (for example,
`Stateful`, `Event`, and `ImageStorePath`) and preserve inherited and subtype
fields. Provision requests must include the subtype's required fields.

Return types follow each operation's documented successes:

- A single bodyless success returns `()`, for example `remove_node_state`.
- A single model response returns its model type.
- A model or empty 204 returns `Option<T>`, for example `get_application_info`.
  Only 204 becomes `None`; malformed model responses remain errors.
- Other multiple-success operations return an operation-specific `...Success`
  enum selected by HTTP status. `ProvisionApplicationTypeSuccess::Status200`
  means completed, while `Status202` means accepted.
  `MeshSecretCreateOrUpdateSuccess::Status200(model)` and `Status201(model)`
  carry the returned model, while `Status202` has no model. Mesh delete enums
  distinguish 200, 202, and 204. Undocumented success statuses are errors for
  enum-returning operations.

Only 2xx responses are successful. Disable redirects on the configured Reqwest
client when the original response must be observed; non-2xx responses are exposed
through `Error::ResponseError`, including their status and body.

`upload_file` and `upload_file_chunk` accept a final `body: Vec<u8>` argument,
sent unchanged as `application/octet-stream`. Pass `Vec::new()` for an empty
`_.dir` marker. Health-state-filter query arguments use `Option<u32>` and accept
the full unsigned 32-bit range without negative-value reinterpretation.

These changes affect source compatibility with 0.1.1. Regenerate with
`bash generation/service-fabric/generate.sh` from the repository root; see
[`docs/Dev.md`](../docs/Dev.md) for the generator extension and validation steps.
The OpenAPI specification is unchanged.