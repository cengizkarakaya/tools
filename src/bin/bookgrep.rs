// `src/bin` altındaki binary, aynı projedeki `src/bookgrep` modülünü bu yolla içeri alır.
#[path = "../bookgrep/mod.rs"]
mod bookgrep;

fn main() -> anyhow::Result<()> {
    // `anyhow::Result` sayesinde `?` ile gelen hatalar kullanıcıya temiz biçimde dönebilir.
    // CLI girişini bookgrep modülündeki ana akışa devreder.
    bookgrep::run()
}
