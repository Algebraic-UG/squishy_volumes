fn main() {
    let compiler = wesl::Wesl::new("src/shaders");

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
