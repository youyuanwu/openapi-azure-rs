#[cfg(test)]
mod tests {
    use openapi_service_fabric::apis::configuration::Configuration;

    // Requires local onebox cluster running
    #[tokio::test]
    async fn it_works_async() {
        let configuration = Configuration {
            base_path: "http://localhost:19080".to_string(),
            ..Default::default()
        };
        let api_version = "6.0";
        let response = openapi_service_fabric::apis::node_api::get_node_info_list(
            &configuration,
            api_version,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("API call failed");

        let nodes = response.items.unwrap();
        assert!(!nodes.is_empty(), "Node list should not be empty");
        println!("Retrieved nodes: {nodes:?}");
    }
}
