use crate::{parse_to_url, JobContext, LoadArgs, PluginDriver, ResolveArgs, TransformArgs};
use nodejs_resolver::ResolveResult;
use std::path::Path;
use sugar_path::PathSugar;

pub async fn load(
    plugin_driver: &PluginDriver,
    args: LoadArgs<'_>,
    job_ctx: &mut JobContext,
) -> anyhow::Result<String> {
    let plugin_output = plugin_driver.load(args.clone(), job_ctx).await?;

    if let Some(output) = plugin_output {
        Ok(output)
    } else {
        Ok(tokio::fs::read_to_string(args.id)
            .await
            .map_err(|_| anyhow::format_err!("fail to load {:?}", args.id))?)
    }
}

pub fn transform(_args: TransformArgs) -> String {
    todo!()
}

pub async fn resolve(
    args: ResolveArgs<'_>,
    plugin_driver: &PluginDriver,
    job_context: &mut JobContext,
) -> anyhow::Result<String> {
    // TODO: plugins

    let plugin_output = plugin_driver.resolve(args.clone(), job_context).await?;

    if let Some(output) = plugin_output {
        return Ok(output);
    }

    // plugin_driver.resolver
    let base_dir = if let Some(importer) = args.importer {
        Path::new(importer)
            .parent()
            .ok_or_else(|| anyhow::format_err!("parent() failed for {:?}", importer))?
    } else {
        Path::new(plugin_driver.options.root.as_str())
    };
    Ok({
        tracing::trace!(
            "resolved importer:{:?},specifier:{:?}",
            args.importer,
            args.specifier
        );
        let mut path = Path::new(base_dir)
            .join(Path::new(args.specifier))
            .resolve();
        path.set_extension("js");
        path.to_string_lossy().to_string()
    })
}
