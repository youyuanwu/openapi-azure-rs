pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works_async() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
