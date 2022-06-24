use core::CompilerOptions;


pub struct Rolldown {
  compiler: core::Compiler,
}

impl Rolldown {
  pub async fn build(&mut self) -> anyhow::Result<()> {
    self.compiler.run().await?;
    Ok(())
  }
}

pub fn rolldown(options: CompilerOptions) -> Rolldown {
  let compiler = core::Compiler::new(options, vec![]);
  Rolldown { compiler }
}