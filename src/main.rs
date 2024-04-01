use anyhow::Ok;

fn main() -> anyhow::Result<()> {
    let mut editor = reditor::Editor::new();

    editor.execute()?;

    Ok(())
}
