fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use std::{future::Future, time::Duration}; // Ensure std::future::Future is imported

    use rstest::{fixture, rstest};
    use testcontainers::{
        ContainerAsync, GenericImage, ImageExt,
        core::{IntoContainerPort, WaitFor},
        runners::AsyncRunner,
    };
    use tokio_postgres::{self, NoTls};

    #[tokio::test]
    async fn test_redis_container() {
        let container = GenericImage::new("redis", "latest")
            .with_exposed_port(6379.tcp())
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .unwrap();

        // Get the mapped port for Redis connection
        let port = container
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get host port mapping for Redis");

        println!("Redis container is running on port {}", port);

        // Create connection URL for Redis
        let redis_url = format!("redis://localhost:{}", port);

        // Add a small delay before attempting to connect
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Connect to Redis using the correct method for Redis 0.31.0
        let client = redis::Client::open(redis_url).expect("Failed to create Redis client");
        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .expect("Failed to connect to Redis");

        // Test simple SET and GET operations
        let _: () = redis::cmd("SET")
            .arg("test_key")
            .arg("test_value")
            .query_async(&mut conn)
            .await
            .expect("Failed to set key");

        let value: String = redis::cmd("GET")
            .arg("test_key")
            .query_async(&mut conn)
            .await
            .expect("Failed to get key");

        // Verify the value
        assert_eq!(
            value, "test_value",
            "Redis GET operation returned unexpected value"
        );

        println!("Successfully connected to Redis container and executed commands");
    }

    #[tokio::test]
    async fn test_postgres_container() {
        let container = GenericImage::new("postgres", "latest")
            .with_exposed_port(5432.tcp())
            .with_wait_for(WaitFor::message_on_stderr(
                "database system is ready to accept connections",
            ))
            .with_env_var("POSTGRES_PASSWORD", "password")
            .with_env_var("POSTGRES_USER", "postgres")
            .with_env_var("POSTGRES_DB", "postgres")
            .start()
            .await
            .unwrap();

        // Get the mapped port for PostgreSQL connection
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("Failed to get host port mapping for PostgreSQL");

        println!("PostgreSQL container is running on port {}", port);

        // Test connection to the database
        let conn_string = format!(
            "host=localhost port={} user=postgres password=password dbname=postgres",
            port
        );

        // Attempt to connect to the database
        let conn_result = tokio_postgres::connect(&conn_string, NoTls).await;

        match conn_result {
            Ok((client, connection)) => {
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("Connection error: {}", e);
                    }
                });

                // Execute a simple query to verify the connection is working
                let rows = client
                    .query("SELECT 1 as result", &[])
                    .await
                    .expect("Failed to execute query");

                let result: i32 = rows[0].get("result");
                assert_eq!(result, 1, "Expected query to return 1");

                println!("Successfully connected to PostgreSQL container and executed query");
            }
            Err(e) => {
                panic!("Failed to connect to PostgreSQL: {}", e);
            }
        }
    }

    #[fixture]
    fn test_container() -> impl Future<Output = ContainerAsync<GenericImage>> + Send {
        async {
            GenericImage::new("postgres", "latest")
                .with_exposed_port(5432.tcp())
                .with_wait_for(WaitFor::message_on_stderr(
                    "database system is ready to accept connections",
                ))
                .with_env_var("POSTGRES_PASSWORD", "password")
                .with_env_var("POSTGRES_USER", "postgres")
                .with_env_var("POSTGRES_DB", "postgres")
                .start()
                .await
                .unwrap()
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_container_two(
        test_container: impl Future<Output = ContainerAsync<GenericImage>> + Send,
    ) {
        let test_container = test_container.await; // Manually await the injected future

        let port = test_container
            .get_host_port_ipv4(5432)
            .await
            .expect("Failed to get host port mapping for PostgreSQL");
        println!("PostgreSQL container is running on port {}", port);

        let conn_string = format!("postgresql://postgres:password@localhost:{}/postgres", port);

        tokio::time::sleep(Duration::from_millis(200)).await; // Increased delay

        let task = tokio::spawn(async move {
            let pool = sqlx::PgPool::connect(&conn_string).await.unwrap();
            let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await.unwrap();

            assert_eq!(row.0, 1, "Expected query to return 1");
        });

        // Wait for the spawned task to complete to ensure the test doesn't end prematurely
        task.await.expect("Task failed");
    }

    #[rstest]
    #[tokio::test]
    async fn test_container_three(
        test_container: impl Future<Output = ContainerAsync<GenericImage>> + Send,
    ) {
        let test_container = test_container.await; // Manually await the injected future

        let port = test_container
            .get_host_port_ipv4(5432)
            .await
            .expect("Failed to get host port mapping for PostgreSQL");
        println!("PostgreSQL container is running on port {}", port);

        let conn_string = format!("postgresql://postgres:password@localhost:{}/postgres", port);

        tokio::time::sleep(Duration::from_millis(200)).await; // Increased delay

        let task = tokio::spawn(async move {
            let pool = sqlx::PgPool::connect(&conn_string).await.unwrap();
            let row: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool).await.unwrap();

            assert_eq!(row.0, 1, "Expected query to return 1");
        });

        // Wait for the spawned task to complete to ensure the test doesn't end prematurely
        task.await.expect("Task failed");
    }
}
