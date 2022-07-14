use std::path::Path;

// use rspack::Compiler;
use hashbrown::HashMap;
use rolldown::{rolldown, Rolldown};
use rolldown_core::NormalizedInputOptions;

pub async fn test_fixture(fixture_path: &Path) -> Rolldown {
    let mut compiler = rolldown(NormalizedInputOptions {
        // input: HashMap::from([("main".to_string(), "./src/index.js".to_string().into())]),
        input: HashMap::from([(
            "main".to_string(),
            fixture_path
                .join("index.js")
                .to_str()
                .unwrap()
                .to_string()
                .into(),
        )]),
        root: fixture_path.to_string_lossy().to_string(),
        ..Default::default()
    });

    let expected_dir_path = fixture_path.join("expected");

    let mut expected = std::fs::read_dir(expected_dir_path)
        .unwrap()
        .flat_map(|entry| entry.ok())
        .filter_map(|entry| {
            let content = std::fs::read_to_string(entry.path()).ok()?;
            Some((entry.file_name().to_string_lossy().to_string(), content))
        })
        .collect::<HashMap<_, _>>();

    let stats = compiler.write(Default::default()).await.unwrap();
    // println!("stats.output {:?}", stats.output);
    stats.output.iter().for_each(|asset| {
        expected
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|filename| {
                if asset.filename.ends_with(&filename) {
                    similar_asserts::assert_eq!(
                        asset.code.trim(),
                        expected.remove(&filename).unwrap().trim(),
                        "Test failed due to the file {:?}",
                        filename
                    )
                };
            });
    });
    assert!(
        expected.is_empty(),
        "files {:?} are not visited",
        expected.keys().collect::<Vec<_>>()
    );
    compiler
}
