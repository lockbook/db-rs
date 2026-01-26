use uuid::Uuid;

pub fn random_test_dir() -> String {
    format!("/tmp/{}", Uuid::new_v4())
}
