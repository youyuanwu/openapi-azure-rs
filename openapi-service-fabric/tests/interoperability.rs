use openapi_service_fabric::{
    apis::{
        application_api, application_type_api, configuration::Configuration, image_store_api,
        mesh_secrets_api, node_api, property_management_api, service_api, Error,
    },
    models::{
        ApplicationPackageCleanupPolicy, HealthEvaluation, NameDescription,
        ProvisionApplicationTypeDescriptionBase, ServiceInfo,
    },
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    thread::{self, JoinHandle},
    time::Duration,
};

fn round_trip<T: DeserializeOwned + Serialize>(value: Value) -> T {
    let model: T = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(&model).unwrap(), value);
    model
}

#[test]
fn service_subtypes_preserve_base_and_derived_fields() {
    for persisted in [false, true] {
        let service = round_trip::<ServiceInfo>(json!({
            "ServiceKind": "Stateful",
            "Name": "fabric:/app/service",
            "HealthState": "Ok",
            "HasPersistedState": persisted
        }));
        assert!(matches!(
            service,
            ServiceInfo::StatefulServiceInfo { has_persisted_state: Some(value), .. }
                if value == persisted
        ));
    }
    let service = round_trip::<ServiceInfo>(json!({
        "ServiceKind": "Stateless",
        "Name": "fabric:/app/stateless",
        "IsServiceGroup": false
    }));
    assert!(matches!(service, ServiceInfo::StatelessServiceInfo { .. }));
}

#[test]
fn health_evaluations_preserve_nested_events_and_policy() {
    let event = json!({
        "Kind": "Event",
        "AggregatedHealthState": "Warning",
        "Description": "example",
        "ConsiderWarningAsError": true,
        "UnhealthyEvent": {
            "SourceId": "watchdog",
            "Property": "disk",
            "HealthState": "Warning",
            "Description": "low disk space",
            "SequenceNumber": "42",
            "IsExpired": false
        }
    });
    round_trip::<HealthEvaluation>(event.clone());
    round_trip::<HealthEvaluation>(json!({
        "Kind": "Replicas",
        "AggregatedHealthState": "Error",
        "TotalCount": 3,
        "MaxPercentUnhealthyReplicasPerPartition": 10,
        "UnhealthyEvaluations": [{"HealthEvaluation": event}]
    }));
}

fn provision_request() -> ProvisionApplicationTypeDescriptionBase {
    ProvisionApplicationTypeDescriptionBase::ProvisionApplicationTypeDescription {
        r#async: false,
        application_type_build_path: "example-package".to_owned(),
        application_package_cleanup_policy: Some(ApplicationPackageCleanupPolicy::Manual),
    }
}

#[test]
fn provisioning_encodes_required_derived_fields() {
    let value = json!({
        "Kind": "ImageStorePath",
        "Async": false,
        "ApplicationTypeBuildPath": "example-package",
        "ApplicationPackageCleanupPolicy": "Manual"
    });
    assert_eq!(serde_json::to_value(provision_request()).unwrap(), value);
    round_trip::<ProvisionApplicationTypeDescriptionBase>(value);
    assert!(
        serde_json::from_value::<ProvisionApplicationTypeDescriptionBase>(json!({
            "Kind": "ImageStorePath", "Async": false
        }))
        .is_err()
    );
    round_trip::<ProvisionApplicationTypeDescriptionBase>(json!({
        "Kind": "ExternalStore",
        "Async": true,
        "ApplicationPackageDownloadUri": "https://example.com/app.sfpkg",
        "ApplicationTypeName": "Example",
        "ApplicationTypeVersion": "1.0"
    }));
}

struct Request {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn serve(
    status: u16,
    content_type: Option<&str>,
    body: &str,
) -> (Configuration, JoinHandle<Request>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let content_type = content_type
        .map(|value| format!("Content-Type: {value}\r\n"))
        .unwrap_or_default();
    let response = format!(
        "HTTP/1.1 {status} Test\r\n{content_type}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let mut parts = line.split_whitespace();
        let method = parts.next().unwrap().to_owned();
        let target = parts.next().unwrap().to_owned();
        let mut headers = Vec::new();
        loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.trim_end().split_once(':').unwrap();
            headers.push((name.to_ascii_lowercase(), value.trim().to_owned()));
        }
        let length = headers
            .iter()
            .find(|(name, _)| name == "content-length")
            .map(|(_, value)| value.parse().unwrap())
            .unwrap_or(0);
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        stream.write_all(response.as_bytes()).unwrap();
        Request {
            method,
            target,
            headers,
            body,
        }
    });
    let configuration = Configuration {
        base_path: format!("http://{address}"),
        client: reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap(),
        ..Default::default()
    };
    (configuration, handle)
}

#[tokio::test]
async fn application_lookup_distinguishes_found_absent_and_invalid() {
    let (config, server) = serve(200, Some("application/json"), r#"{"Id":"app"}"#);
    let application = application_api::get_application_info(&config, "8.2", "app", None, None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(application.id.as_deref(), Some("app"));
    server.join().unwrap();

    for content_type in [None, Some("application/json"), Some("text/plain")] {
        let (config, server) = serve(204, content_type, "");
        let application =
            application_api::get_application_info(&config, "8.2", "missing", None, None)
                .await
                .unwrap();
        assert!(application.is_none());
        server.join().unwrap();
    }
    for body in ["", "{", r#"{"Status":123}"#] {
        let (config, server) = serve(200, Some("application/json"), body);
        let result = application_api::get_application_info(&config, "8.2", "app", None, None).await;
        assert!(matches!(result, Err(Error::Serde(_))), "{result:?}");
        server.join().unwrap();
    }
    let (config, server) = serve(200, None, "");
    assert!(matches!(
        application_api::get_application_info(&config, "8.2", "app", None, None).await,
        Err(Error::Serde(_))
    ));
    server.join().unwrap();
}

#[tokio::test]
async fn application_lookup_preserves_http_errors() {
    for status in [302, 404, 500] {
        let body = r#"{"Error":{"Code":"FABRIC_E_APPLICATION_NOT_FOUND","Message":"missing"}}"#;
        let (config, server) = serve(status, Some("application/json"), body);
        let error = application_api::get_application_info(&config, "8.2", "app", None, None)
            .await
            .unwrap_err();
        match error {
            Error::ResponseError(response) => {
                assert_eq!(response.status.as_u16(), status);
                assert_eq!(response.content, body);
                assert!(response.entity.is_some());
            }
            other => panic!("{other:?}"),
        }
        server.join().unwrap();
    }
}

#[tokio::test]
async fn model_responses_distinguish_completed_created_and_accepted() {
    use mesh_secrets_api::MeshSecretCreateOrUpdateSuccess;

    for status in [200, 201] {
        let body = r#"{"name":"example","properties":{"kind":"inlinedValue"}}"#;
        let (config, server) = serve(status, Some("application/json"), body);
        let outcome = mesh_secrets_api::mesh_secret_create_or_update(
            &config,
            "6.4-preview",
            "example",
            serde_json::from_str(body).unwrap(),
        )
        .await
        .unwrap();
        let model = match (outcome, status) {
            (MeshSecretCreateOrUpdateSuccess::Status200(model), 200)
            | (MeshSecretCreateOrUpdateSuccess::Status201(model), 201) => model,
            other => panic!("Wrong success outcome: {other:?}"),
        };
        assert_eq!(model.name, "example");
        server.join().unwrap();
    }
    for content_type in [None, Some("application/json"), Some("text/plain")] {
        let (config, server) = serve(202, content_type, "");
        let outcome = mesh_secrets_api::mesh_secret_create_or_update(
            &config,
            "6.4-preview",
            "example",
            Default::default(),
        )
        .await
        .unwrap();
        assert_eq!(outcome, MeshSecretCreateOrUpdateSuccess::Status202);
        server.join().unwrap();
    }
    for status in [200, 201] {
        for body in ["", "{", r#"{"name":"example"}"#] {
            let (config, server) = serve(status, Some("application/json"), body);
            let result = mesh_secrets_api::mesh_secret_create_or_update(
                &config,
                "6.4-preview",
                "example",
                Default::default(),
            )
            .await;
            assert!(matches!(result, Err(Error::Serde(_))), "{result:?}");
            server.join().unwrap();
        }
        for content_type in [None, Some("text/plain"), Some("application/octet-stream")] {
            let body = r#"{"name":"example","properties":{"kind":"inlinedValue"}}"#;
            let (config, server) = serve(status, content_type, body);
            let result = mesh_secrets_api::mesh_secret_create_or_update(
                &config,
                "6.4-preview",
                "example",
                Default::default(),
            )
            .await;
            assert!(matches!(result, Err(Error::Serde(_))), "{result:?}");
            server.join().unwrap();
        }
    }
    for status in [203, 204, 206, 302, 404, 500] {
        let body = if status == 204 {
            ""
        } else {
            "Unexpected response"
        };
        let (config, server) = serve(status, None, body);
        let result = mesh_secrets_api::mesh_secret_create_or_update(
            &config,
            "6.4-preview",
            "example",
            Default::default(),
        )
        .await;
        assert!(matches!(result, Err(Error::ResponseError(response))
            if response.status.as_u16() == status && response.content == body));
        server.join().unwrap();
    }
}

#[tokio::test]
async fn service_api_returns_complete_subtype() {
    let body = r#"{"ServiceKind":"Stateful","HasPersistedState":false}"#;
    let (config, server) = serve(200, Some("application/json"), body);
    let service = service_api::get_service_info(&config, "app", "app~service", "8.2", None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(service).unwrap(),
        json!({
            "ServiceKind": "Stateful", "HasPersistedState": false
        })
    );
    server.join().unwrap();
}

#[tokio::test]
async fn provisioning_api_sends_complete_subtype() {
    use application_type_api::ProvisionApplicationTypeSuccess;

    for (status, expected) in [
        (200, ProvisionApplicationTypeSuccess::Status200),
        (202, ProvisionApplicationTypeSuccess::Status202),
    ] {
        let (config, server) = serve(status, None, "");
        let outcome = application_type_api::provision_application_type(
            &config,
            "8.2",
            provision_request(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(outcome, expected);
        let request = server.join().unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&request.body).unwrap(),
            serde_json::to_value(provision_request()).unwrap()
        );
    }
    for status in [201, 204, 206, 302, 404, 500] {
        let body = if status == 204 {
            ""
        } else {
            "Unexpected response"
        };
        let (config, server) = serve(status, None, body);
        let result = application_type_api::provision_application_type(
            &config,
            "8.2",
            provision_request(),
            None,
        )
        .await;
        assert!(matches!(result, Err(Error::ResponseError(response))
            if response.status.as_u16() == status && response.content == body));
        server.join().unwrap();
    }
}

#[tokio::test]
async fn upload_sends_binary_contents_and_empty_directory_markers() {
    for (path, bytes) in [
        ("package/file", vec![0, 1, 127, 128, 255, b'\n']),
        ("package/_.dir", vec![]),
    ] {
        let (config, server) = serve(200, None, "");
        let () = image_store_api::upload_file(&config, "8.2", path, None, bytes.clone())
            .await
            .unwrap();
        let request = server.join().unwrap();
        assert_eq!(request.method, "PUT");
        assert_eq!(
            request.target,
            format!(
                "/ImageStore/{}?api-version=8.2",
                openapi_service_fabric::apis::urlencode(path)
            )
        );
        assert_eq!(request.body, bytes);
        assert!(request.headers.contains(&(
            "content-type".to_owned(),
            "application/octet-stream".to_owned()
        )));
    }
}

#[tokio::test]
async fn upload_chunk_sends_binary_contents() {
    let (config, server) = serve(200, None, "");
    image_store_api::upload_file_chunk(
        &config,
        "8.2",
        "file",
        "session",
        "bytes 0-2/3",
        None,
        vec![0, 128, 255],
    )
    .await
    .unwrap();
    let request = server.join().unwrap();
    assert_eq!(request.body, [0, 128, 255]);
    assert!(request
        .headers
        .contains(&("content-range".to_owned(), "bytes 0-2/3".to_owned())));
}

#[tokio::test]
async fn health_filters_preserve_the_full_unsigned_range() {
    for mask in [0, 65535, 2147483648, u32::MAX] {
        let (config, server) = serve(200, Some("application/json"), "{}");
        node_api::get_node_health(&config, "8.2", "node", Some(mask), None)
            .await
            .unwrap();
        assert!(server
            .join()
            .unwrap()
            .target
            .ends_with(&format!("EventsHealthStateFilter={mask}")));
        let (config, server) = serve(200, Some("application/json"), "{}");
        node_api::get_node_health_using_policy(&config, "8.2", "node", Some(mask), None, None)
            .await
            .unwrap();
        assert!(server
            .join()
            .unwrap()
            .target
            .ends_with(&format!("EventsHealthStateFilter={mask}")));
    }
}

#[tokio::test]
async fn single_bodyless_operations_return_unit_and_reject_redirects() {
    for status in [200, 201, 202, 204, 301, 302, 307, 308, 404, 500] {
        let (config, server) = serve(status, None, "");
        let result = property_management_api::create_name(
            &config,
            "8.2",
            NameDescription::new("fabric:/name".to_owned()),
            None,
        )
        .await;
        if (200..300).contains(&status) {
            let () = result.unwrap();
        } else {
            assert!(matches!(result, Err(Error::ResponseError(response))
                if response.status.as_u16() == status));
        }
        server.join().unwrap();

        let (config, server) = serve(status, None, "");
        let result = property_management_api::delete_name(&config, "8.2", "name", None).await;
        if (200..300).contains(&status) {
            let () = result.unwrap();
        } else {
            assert!(matches!(result, Err(Error::ResponseError(response))
                if response.status.as_u16() == status));
        }
        server.join().unwrap();

        let (config, server) = serve(status, None, "");
        let result =
            property_management_api::get_name_exists_info(&config, "8.2", "name", None).await;
        if (200..300).contains(&status) {
            let () = result.unwrap();
        } else {
            assert!(matches!(result, Err(Error::ResponseError(response))
                if response.status.as_u16() == status));
        }
        server.join().unwrap();
    }
}

#[tokio::test]
async fn remove_node_state_returns_unit_and_preserves_http_errors() {
    let (config, server) = serve(200, None, "");
    let () = node_api::remove_node_state(&config, "8.2", "node", None)
        .await
        .unwrap();
    let request = server.join().unwrap();
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.target,
        "/Nodes/node/$/RemoveNodeState?api-version=8.2"
    );

    for status in [301, 302, 307, 308, 400, 404, 500] {
        let body = r#"{"Error":{"Code":"FABRIC_E_NODE_NOT_FOUND","Message":"missing"}}"#;
        let (config, server) = serve(status, Some("application/json"), body);
        let result = node_api::remove_node_state(&config, "8.2", "node", None).await;
        match result {
            Err(Error::ResponseError(response)) => {
                assert_eq!(response.status.as_u16(), status);
                assert_eq!(response.content, body);
                assert!(response.entity.is_some());
            }
            other => panic!("Expected HTTP error for {status}: {other:?}"),
        }
        server.join().unwrap();
    }
}

#[tokio::test]
async fn mesh_delete_distinguishes_completed_accepted_and_absent() {
    use mesh_secrets_api::MeshSecretDeleteSuccess;

    for (status, expected) in [
        (200, MeshSecretDeleteSuccess::Status200),
        (202, MeshSecretDeleteSuccess::Status202),
        (204, MeshSecretDeleteSuccess::Status204),
    ] {
        let (config, server) = serve(status, None, "");
        let outcome = mesh_secrets_api::mesh_secret_delete(&config, "6.4-preview", "example")
            .await
            .unwrap();
        assert_eq!(outcome, expected);
        server.join().unwrap();
    }
    for status in [201, 206, 301, 307, 404, 500] {
        let body = "Unexpected response";
        let (config, server) = serve(status, None, body);
        let result = mesh_secrets_api::mesh_secret_delete(&config, "6.4-preview", "example").await;
        assert!(matches!(result, Err(Error::ResponseError(response))
            if response.status.as_u16() == status && response.content == body));
        server.join().unwrap();
    }
}
