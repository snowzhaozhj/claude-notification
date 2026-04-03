pub trait Dispatcher: Send + Sync {
    fn dispatch(&self, title: &str, body: &str) -> Result<(), String>;
}
