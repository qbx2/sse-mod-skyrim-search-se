use anyhow::Context;

mod npc;

pub(crate) unsafe fn init(image_base: usize) -> anyhow::Result<()> {
    npc::init(image_base).context("npc::init")?;

    Ok(())
}
