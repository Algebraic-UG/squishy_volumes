use std::io::BufRead as _;

use wesl::Wesl;
use wgcore::Shader as _;

fn filter_lines_with_hashtag(filename: impl AsRef<std::path::Path> + std::fmt::Debug) -> String {
    println!("reading {filename:?}");
    let file = std::fs::File::open(filename).unwrap();
    let mut result = String::new();
    for line in std::io::BufReader::new(file).lines() {
        let line = line.unwrap();
        if line.starts_with("#") {
            continue;
        }
        result.push_str(&line);
        result.push('\n');
    }
    result
}

fn main() {
    let quat_source = filter_lines_with_hashtag(
        wgebra::WgQuat::absolute_path().expect("couldn't find quat source"),
    );
    let mut svd_source = "import package::wgebra::quat as Quat;\n".to_string();
    svd_source.push_str(&filter_lines_with_hashtag(
        wgebra::WgSvd3::absolute_path().expect("couldn't find svd3 source"),
    ));
    println!("{svd_source}");

    let mut resolver = wesl::VirtualResolver::new();
    resolver.add_module("package::quat".parse().unwrap(), quat_source.into());
    resolver.add_module("package::svd3".parse().unwrap(), svd_source.into());

    let mut router = wesl::Router::new();
    router.mount_resolver("package::wgebra".parse().unwrap(), resolver);
    router.mount_fallback_resolver(wesl::FileResolver::new("src/shaders"));
    let compiler = Wesl::new("").set_custom_resolver(router);

    for shader_path in std::fs::read_dir("src/shaders")
        .unwrap()
        .filter_map(|res| res.ok())
        .map(|dir_entry| dir_entry.path())
        .filter_map(|path| {
            path.extension()
                .is_some_and(|ext| ext == "wesl")
                .then_some(path)
        })
    {
        println!("Found shader: {shader_path:?}");
        let name = shader_path.file_stem().unwrap().to_str().unwrap();
        compiler.build_artifact(&format!("package::{name}",).parse().unwrap(), name);
    }
}
