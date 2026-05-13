#[path = "../bookgrep/mod.rs"]
mod bookgrep;

fn main() -> anyhow::Result<()> {
    bookgrep::run()
}
