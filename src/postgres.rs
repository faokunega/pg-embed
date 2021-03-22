use futures::future::BoxFuture;
use futures::AsyncWriteExt;
use std::process::Command;

///
/// Create database password file
///
pub fn create_password_file<'a>(
    location: &'a str,
    password: &'a str,
) -> BoxFuture<'a, anyhow::Result<()>> {
    Box::pin(async move {
        let file_path = format!(
            "{}/{}",
            location, "pwfile"
        );
        let mut file = async_std::fs::File::create(&file_path).await?;
        let _ = file
            .write(password.as_bytes())
            .await?;
        Ok(())
    })
}

///
/// Initialize postgresql database
///
pub fn init_db(
    location: &str,
    user_name: &str,
) -> anyhow::Result<()> {
    Command::new(
        "./data/postgres/bin/initdb",
    )
    .args(&[
        "-A",
        "password",
        "-U",
        user_name,
        "postgres",
        "-D",
        location,
        "--pwfile=data/pwfile",
    ])
    .output()
    .expect(
        "failed to execute process",
    );
    Ok(())
}

///
/// Start postgresql database
///
pub fn start_db(
    location: &str,
) -> anyhow::Result<()> {
    Command::new(
        "./data/postgres/bin/pg_ctl",
    )
    .args(&[
        "start", "-w", "-D", location,
    ])
    .output()
    .expect(
        "failed to execute process",
    );
    Ok(())
}

#[cfg(test)]
mod postgres_tests {
    use super::*;

    #[test]
    fn postgres_start(
    ) -> anyhow::Result<()> {
        start_db("data/db")
    }

    #[async_std::test]
    async fn password_file_creation(
    ) -> anyhow::Result<()> {
        create_password_file(
            "data", "password",
        )
        .await
    }

    #[test]
    fn database_initialization(
    ) -> anyhow::Result<()> {
        init_db("data/db", "postgres")
    }
}
