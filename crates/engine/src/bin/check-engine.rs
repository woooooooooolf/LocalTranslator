fn main() {
    let root = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .expect("pass the external models directory");
    jingyi_engine::run_stdio(root);
}
