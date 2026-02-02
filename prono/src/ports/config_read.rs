pub trait ReadConfig<T> {
    fn default_config_path() -> std::path::PathBuf;
    fn read<P: AsRef<std::path::Path>>(config: P) -> T;
}
